use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;
use std::num::NonZeroU32;
use std::path::Path;

/// Wraps a loaded GGUF model. Created once, kept alive for the session.
/// Thread-safe: owned by the smart-predict worker thread.
pub struct SmartEngine {
    backend: LlamaBackend,
    model: LlamaModel,
    n_threads: u32,
}

impl SmartEngine {
    /// Load a GGUF model from disk. Blocks for 1–3s — always call from a
    /// background thread, never from the key-event handler.
    pub fn load(model_path: &Path, n_threads: u32) -> anyhow::Result<Self> {
        let backend = LlamaBackend::init()?;

        let model_params = LlamaModelParams::default()
            .with_n_gpu_layers(0); // CPU-only; remove to enable Metal/CUDA

        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)?;

        Ok(Self { backend, model, n_threads })
    }

    /// Predict the correctly-accented Vietnamese word for `raw_word` given the
    /// already-typed Vietnamese `context`. Returns None on any error.
    pub fn predict(&self, raw_word: &str, context: &str) -> Option<String> {
        let prompt = build_prompt(raw_word, context);

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(256).unwrap())
            .with_n_threads(self.n_threads)
            .with_n_threads_batch(self.n_threads);

        let mut ctx = self.model.new_context(&self.backend, ctx_params).ok()?;

        // Tokenize the full prompt
        let tokens = self
            .model
            .str_to_token(&prompt, AddBos::Always)
            .ok()?;

        if tokens.is_empty() {
            return None;
        }

        // Feed all prompt tokens — mark only the last one as needing logits
        let mut batch = LlamaBatch::new(512, 1);
        let n = tokens.len();
        for (i, &token) in tokens.iter().enumerate() {
            batch.add(token, i as i32, &[0], i == n - 1).ok()?;
        }
        ctx.decode(&mut batch).ok()?;

        // Greedy sampler with low temperature for deterministic single-word output
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(0.1),
            LlamaSampler::greedy(),
        ]);

        let mut result = String::new();
        let mut n_pos = tokens.len() as i32;

        // Cap at 12 tokens — a single Vietnamese word never exceeds this
        for _ in 0..12 {
            let token = sampler.sample(&ctx, -1);

            if self.model.is_eog_token(token) {
                break;
            }

            let piece = self
                .model
                .token_to_str(token, Special::Tokenize)
                .ok()?;

            // Stop at newline — model is instructed to output one word per line
            if piece.contains('\n') {
                break;
            }

            result.push_str(&piece);

            // Decode the next position
            batch.clear();
            batch.add(token, n_pos, &[0], true).ok()?;
            ctx.decode(&mut batch).ok()?;
            n_pos += 1;
        }

        let word = result.trim().to_string();

        // Sanity checks: reject empty or multi-word output (model failure modes)
        if word.is_empty() || word.split_whitespace().count() > 2 {
            return None;
        }

        Some(word)
    }
}

/// Build the Qwen2.5-Instruct chat-template prompt.
///
/// Few-shot examples are included for the most ambiguous short Vietnamese words —
/// essential at 1.5B parameters where the model needs explicit nudges.
fn build_prompt(raw_word: &str, context: &str) -> String {
    let system = "Bạn là trợ lý bộ gõ tiếng Việt. \
Người dùng gõ từng từ tiếng Việt không dấu (la, tieng, viet...). \
Cho ngữ cảnh câu đã gõ (tiếng Việt có dấu) và từ tiếp theo chưa có dấu, \
hãy trả lời CHỈ duy nhất một từ tiếng Việt đúng dấu. \
Không giải thích. Không dấu câu. Chỉ một từ duy nhất.\n\n\
Ví dụ:\n\
Ngữ cảnh: (không)     Từ: toi    → tôi\n\
Ngữ cảnh: tôi         Từ: la     → là\n\
Ngữ cảnh: tôi là      Từ: nguoi  → người\n\
Ngữ cảnh: đây         Từ: la     → là\n\
Ngữ cảnh: đây là      Từ: tieng  → tiếng\n\
Ngữ cảnh: tiếng       Từ: viet   → việt\n\
Ngữ cảnh: anh         Từ: co     → có\n\
Ngữ cảnh: không       Từ: co     → có\n\
Ngữ cảnh: cô ấy       Từ: da     → đã\n\
Ngữ cảnh: họ          Từ: di     → đi\n\
Ngữ cảnh: con         Từ: ma     → ma\n\
Ngữ cảnh: (không)     Từ: bay    → bay";

    let user = if context.is_empty() {
        format!("Ngữ cảnh: (không)\nTừ: {raw_word}")
    } else {
        format!("Ngữ cảnh: {context}\nTừ: {raw_word}")
    };

    // Qwen2.5-Instruct chat template
    format!(
        "<|im_start|>system\n{system}<|im_end|>\n\
         <|im_start|>user\n{user}<|im_end|>\n\
         <|im_start|>assistant\n"
    )
}
