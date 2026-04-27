use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    const MAX_QUESTIONS_PER_EXAM: usize = 16;
    const CORRECTNESS_MASKS: [u32; MAX_QUESTIONS_PER_EXAM] = [
        1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
    ];

    pub struct AnswerKey {
        answers: [u8; MAX_QUESTIONS_PER_EXAM],
    }

    pub struct StudentAnswers {
        answers: [u8; MAX_QUESTIONS_PER_EXAM],
        question_count: u8,
    }

    #[instruction]
    pub fn grade_exam_v4(
        answer_key_ctxt: Enc<Shared, AnswerKey>,
        submission: StudentAnswers,
    ) -> (u16, u32) {
        let answer_key = answer_key_ctxt.to_arcis();
        let mut score = 0u16;
        let mut correctness_mask = 0u32;

        for i in 0..MAX_QUESTIONS_PER_EXAM {
            let in_range = (i as u8) < submission.question_count;
            let is_correct = in_range && submission.answers[i] == answer_key.answers[i];

            if is_correct {
                score += 1;
                correctness_mask += CORRECTNESS_MASKS[i];
            }
        }

        (score.reveal(), correctness_mask.reveal())
    }
}
