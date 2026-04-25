use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CallbackAccount;

declare_id!("Ch5KUtPipgBTnjCVX1du7keV7pd6cdxJDLovRErFuSh");

const COMP_DEF_OFFSET_GRADE_EXAM: u32 = comp_def_offset("grade_exam");

pub const GLOBAL_CONFIG_SEED: &[u8] = b"global-config";
pub const USER_SEED: &[u8] = b"user";
pub const COURSE_SEED: &[u8] = b"course";
pub const ENROLLMENT_SEED: &[u8] = b"enrollment";
pub const EXAM_SEED: &[u8] = b"exam";
pub const EXAM_ACCESS_SEED: &[u8] = b"exam-access";
pub const SESSION_SEED: &[u8] = b"session";

pub const MAX_NAME_LEN: usize = 64;
pub const MAX_COURSE_TITLE_LEN: usize = 100;
pub const MAX_EXAM_TITLE_LEN: usize = 100;
pub const MAX_QUESTIONS_PER_EXAM: usize = 16;
pub const MAX_EXAM_CONTENT_CIPHERTEXTS: usize = 256;
pub const MAX_CONTENT_KEY_CIPHERTEXTS: usize = 8;

#[arcium_program]
pub mod proof_arcium {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global_config = &mut ctx.accounts.global_config;
        global_config.authority = ctx.accounts.authority.key();
        global_config.course_counter = 0;
        global_config.exam_counter = 0;
        global_config.bump = ctx.bumps.global_config;
        Ok(())
    }

    pub fn init_grade_exam_comp_def(ctx: Context<InitGradeExamCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn register_user(ctx: Context<RegisterUser>, name: String, role: Role) -> Result<()> {
        validate_string_len(&name, MAX_NAME_LEN, ErrorCode::NameTooLong)?;

        let user = &mut ctx.accounts.user;
        user.authority = ctx.accounts.authority.key();
        user.name = name.clone();
        user.role = role.clone();
        user.bump = ctx.bumps.user;

        emit!(UserRegistered {
            authority: user.authority,
            name,
            role,
        });

        Ok(())
    }

    pub fn create_course(ctx: Context<CreateCourse>, course_id: u64, title: String) -> Result<()> {
        validate_string_len(&title, MAX_COURSE_TITLE_LEN, ErrorCode::CourseTitleTooLong)?;
        require!(
            ctx.accounts.tutor_profile.role == Role::Tutor,
            ErrorCode::TutorOnly
        );

        let global_config = &mut ctx.accounts.global_config;
        let next_course_id = global_config
            .course_counter
            .checked_add(1)
            .ok_or(ErrorCode::MathOverflow)?;
        require_eq!(course_id, next_course_id, ErrorCode::InvalidCourseId);

        let course = &mut ctx.accounts.course;
        course.course_id = course_id;
        course.title = title.clone();
        course.tutor = ctx.accounts.tutor.key();
        course.tutor_name = ctx.accounts.tutor_profile.name.clone();
        course.active = true;
        course.bump = ctx.bumps.course;

        global_config.course_counter = next_course_id;

        emit!(CourseCreated {
            course_id,
            title,
            tutor: course.tutor,
        });

        Ok(())
    }

    pub fn enroll_in_course(ctx: Context<EnrollInCourse>) -> Result<()> {
        require!(
            ctx.accounts.student_profile.role == Role::Student,
            ErrorCode::StudentOnly
        );
        require!(ctx.accounts.course.active, ErrorCode::CourseInactive);

        let enrollment = &mut ctx.accounts.enrollment;
        enrollment.course_id = ctx.accounts.course.course_id;
        enrollment.student = ctx.accounts.student.key();
        enrollment.enrolled_at = Clock::get()?.unix_timestamp;
        enrollment.bump = ctx.bumps.enrollment;

        emit!(StudentEnrolled {
            course_id: enrollment.course_id,
            student: enrollment.student,
        });

        Ok(())
    }

    pub fn create_exam(
        ctx: Context<CreateExam>,
        exam_id: u64,
        title: String,
        question_count: u8,
        encrypted_exam: EncryptedExamInput,
    ) -> Result<()> {
        validate_string_len(&title, MAX_EXAM_TITLE_LEN, ErrorCode::ExamTitleTooLong)?;
        require!(
            ctx.accounts.tutor_profile.role == Role::Tutor,
            ErrorCode::TutorOnly
        );
        require!(ctx.accounts.course.active, ErrorCode::CourseInactive);
        require_keys_eq!(
            ctx.accounts.course.tutor,
            ctx.accounts.tutor.key(),
            ErrorCode::UnauthorizedTutor
        );
        require!(
            question_count > 0 && question_count as usize <= MAX_QUESTIONS_PER_EXAM,
            ErrorCode::QuestionCountOutOfRange
        );
        require!(
            encrypted_exam.content_ciphertexts.len() <= MAX_EXAM_CONTENT_CIPHERTEXTS,
            ErrorCode::EncryptedExamContentOutOfRange
        );
        require_eq!(
            encrypted_exam.answer_key_ciphertexts.len(),
            question_count as usize,
            ErrorCode::InvalidAnswerKeyLength
        );

        let global_config = &mut ctx.accounts.global_config;
        let next_exam_id = global_config
            .exam_counter
            .checked_add(1)
            .ok_or(ErrorCode::MathOverflow)?;
        require_eq!(exam_id, next_exam_id, ErrorCode::InvalidExamId);

        let exam = &mut ctx.accounts.exam;
        exam.exam_id = exam_id;
        exam.course_id = ctx.accounts.course.course_id;
        exam.tutor = ctx.accounts.tutor.key();
        exam.title = title.clone();
        exam.question_count = question_count;
        exam.active = true;
        exam.content_pubkey = encrypted_exam.content_pubkey;
        exam.content_nonce = encrypted_exam.content_nonce;
        exam.content_ciphertexts = encrypted_exam.content_ciphertexts;
        exam.answer_key_nonce = encrypted_exam.answer_key_nonce;
        exam.answer_key_ciphertexts = encrypted_exam.answer_key_ciphertexts;
        exam.bump = ctx.bumps.exam;

        global_config.exam_counter = next_exam_id;

        emit!(ExamCreated {
            exam_id,
            course_id: exam.course_id,
            tutor: exam.tutor,
            question_count,
        });

        Ok(())
    }

    pub fn request_exam_access(
        ctx: Context<RequestExamAccess>,
        student_content_pubkey: [u8; 32],
    ) -> Result<()> {
        require!(
            ctx.accounts.student_profile.role == Role::Student,
            ErrorCode::StudentOnly
        );
        require!(ctx.accounts.course.active, ErrorCode::CourseInactive);
        require!(ctx.accounts.exam.active, ErrorCode::ExamInactive);
        require_eq!(
            ctx.accounts.exam.course_id,
            ctx.accounts.course.course_id,
            ErrorCode::CourseExamMismatch
        );

        let exam_access = &mut ctx.accounts.exam_access;
        exam_access.exam_id = ctx.accounts.exam.exam_id;
        exam_access.course_id = ctx.accounts.course.course_id;
        exam_access.student = ctx.accounts.student.key();
        exam_access.student_content_pubkey = student_content_pubkey;
        exam_access.content_key_nonce = 0;
        exam_access.encrypted_content_key = Vec::new();
        exam_access.granted = true;
        exam_access.bump = ctx.bumps.exam_access;

        emit!(ExamAccessRequested {
            exam_id: exam_access.exam_id,
            course_id: exam_access.course_id,
            student: exam_access.student,
        });

        Ok(())
    }

    pub fn grant_exam_access(
        ctx: Context<GrantExamAccess>,
        encrypted_content_key: EncryptedContentKeyInput,
    ) -> Result<()> {
        require!(
            ctx.accounts.tutor_profile.role == Role::Tutor,
            ErrorCode::TutorOnly
        );
        require!(ctx.accounts.course.active, ErrorCode::CourseInactive);
        require!(ctx.accounts.exam.active, ErrorCode::ExamInactive);
        require_keys_eq!(
            ctx.accounts.course.tutor,
            ctx.accounts.tutor.key(),
            ErrorCode::UnauthorizedTutor
        );
        require_eq!(
            ctx.accounts.exam.course_id,
            ctx.accounts.course.course_id,
            ErrorCode::CourseExamMismatch
        );
        require!(
            !encrypted_content_key.ciphertexts.is_empty()
                && encrypted_content_key.ciphertexts.len() <= MAX_CONTENT_KEY_CIPHERTEXTS,
            ErrorCode::EncryptedContentKeyOutOfRange
        );

        let exam_access = &mut ctx.accounts.exam_access;
        require_eq!(
            exam_access.exam_id,
            ctx.accounts.exam.exam_id,
            ErrorCode::ExamAccessMismatch
        );
        require_eq!(
            exam_access.course_id,
            ctx.accounts.course.course_id,
            ErrorCode::ExamAccessMismatch
        );
        require_eq!(
            exam_access.student,
            ctx.accounts.enrollment.student,
            ErrorCode::ExamAccessMismatch
        );

        exam_access.content_key_nonce = encrypted_content_key.nonce;
        exam_access.encrypted_content_key = encrypted_content_key.ciphertexts;
        exam_access.granted = true;

        emit!(ExamAccessGranted {
            exam_id: exam_access.exam_id,
            course_id: exam_access.course_id,
            student: exam_access.student,
        });

        Ok(())
    }

    pub fn take_exam(
        ctx: Context<TakeExam>,
        computation_offset: u64,
        answers: Vec<u8>,
    ) -> Result<()> {
        require!(
            ctx.accounts.student_profile.role == Role::Student,
            ErrorCode::StudentOnly
        );
        require!(ctx.accounts.course.active, ErrorCode::CourseInactive);
        require!(ctx.accounts.exam.active, ErrorCode::ExamInactive);
        require_eq!(
            ctx.accounts.exam.course_id,
            ctx.accounts.course.course_id,
            ErrorCode::CourseExamMismatch
        );
        require_eq!(
            answers.len(),
            ctx.accounts.exam.question_count as usize,
            ErrorCode::InvalidAnswerCount
        );
        require!(
            ctx.accounts.exam_access.granted,
            ErrorCode::ExamAccessNotGranted
        );

        let mut answer_key = [[0u8; 32]; MAX_QUESTIONS_PER_EXAM];
        for (idx, ciphertext) in ctx.accounts.exam.answer_key_ciphertexts.iter().enumerate() {
            answer_key[idx] = *ciphertext;
        }

        let mut submission = [0u8; MAX_QUESTIONS_PER_EXAM];
        for (idx, answer) in answers.iter().enumerate() {
            require!(*answer <= 3, ErrorCode::AnswerOutOfRange);
            submission[idx] = *answer;
        }

        let session_key = ctx.accounts.session.key();
        let exam_id = ctx.accounts.exam.exam_id;
        let course_id = ctx.accounts.course.course_id;
        let student = ctx.accounts.student.key();
        {
            let session = &mut ctx.accounts.session;
            session.exam_id = exam_id;
            session.course_id = course_id;
            session.student = student;
            session.answers = answers;
            session.score = 0;
            session.correctness_mask = 0;
            session.correctness = vec![false; ctx.accounts.exam.question_count as usize];
            session.completed = false;
            session.completed_at = 0;
            session.bump = ctx.bumps.session;
        }

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let args = ArgBuilder::new()
            .plaintext_u128(ctx.accounts.exam.answer_key_nonce)
            .encrypted_u8(answer_key[0])
            .encrypted_u8(answer_key[1])
            .encrypted_u8(answer_key[2])
            .encrypted_u8(answer_key[3])
            .encrypted_u8(answer_key[4])
            .encrypted_u8(answer_key[5])
            .encrypted_u8(answer_key[6])
            .encrypted_u8(answer_key[7])
            .encrypted_u8(answer_key[8])
            .encrypted_u8(answer_key[9])
            .encrypted_u8(answer_key[10])
            .encrypted_u8(answer_key[11])
            .encrypted_u8(answer_key[12])
            .encrypted_u8(answer_key[13])
            .encrypted_u8(answer_key[14])
            .encrypted_u8(answer_key[15])
            .plaintext_u8(submission[0])
            .plaintext_u8(submission[1])
            .plaintext_u8(submission[2])
            .plaintext_u8(submission[3])
            .plaintext_u8(submission[4])
            .plaintext_u8(submission[5])
            .plaintext_u8(submission[6])
            .plaintext_u8(submission[7])
            .plaintext_u8(submission[8])
            .plaintext_u8(submission[9])
            .plaintext_u8(submission[10])
            .plaintext_u8(submission[11])
            .plaintext_u8(submission[12])
            .plaintext_u8(submission[13])
            .plaintext_u8(submission[14])
            .plaintext_u8(submission[15])
            .plaintext_u8(ctx.accounts.exam.question_count)
            .build();

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![GradeExamCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[CallbackAccount {
                    pubkey: session_key,
                    is_writable: true,
                }],
            )?],
            1,
            0,
        )?;

        emit!(ExamSubmitted {
            exam_id,
            course_id,
            student,
        });

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "grade_exam")]
    pub fn grade_exam_callback(
        ctx: Context<GradeExamCallback>,
        output: SignedComputationOutputs<GradeExamOutput>,
    ) -> Result<()> {
        let result = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(GradeExamOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let score = result.field_0;
        let correctness_mask = result.field_1;
        let session = &mut ctx.accounts.session;
        let question_count = session.answers.len();
        let mut correctness = Vec::with_capacity(question_count);
        for idx in 0..question_count {
            correctness.push(((correctness_mask >> idx) & 1) == 1);
        }

        session.score = score;
        session.correctness_mask = correctness_mask;
        session.correctness = correctness;
        session.completed = true;
        session.completed_at = Clock::get()?.unix_timestamp;

        emit!(ExamCompleted {
            exam_id: session.exam_id,
            course_id: session.course_id,
            student: session.student,
            score,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = GlobalConfig::INIT_SPACE,
        seeds = [GLOBAL_CONFIG_SEED],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterUser<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = User::INIT_SPACE,
        seeds = [USER_SEED, authority.key().as_ref()],
        bump
    )]
    pub user: Account<'info, User>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(course_id: u64, _title: String)]
pub struct CreateCourse<'info> {
    #[account(mut)]
    pub tutor: Signer<'info>,
    #[account(
        seeds = [USER_SEED, tutor.key().as_ref()],
        bump = tutor_profile.bump
    )]
    pub tutor_profile: Account<'info, User>,
    #[account(
        mut,
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    #[account(
        init,
        payer = tutor,
        space = Course::INIT_SPACE,
        seeds = [COURSE_SEED, &course_id.to_le_bytes()],
        bump
    )]
    pub course: Account<'info, Course>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct EnrollInCourse<'info> {
    #[account(mut)]
    pub student: Signer<'info>,
    #[account(
        seeds = [USER_SEED, student.key().as_ref()],
        bump = student_profile.bump
    )]
    pub student_profile: Box<Account<'info, User>>,
    #[account(
        seeds = [COURSE_SEED, &course.course_id.to_le_bytes()],
        bump = course.bump,
        constraint = course.active @ ErrorCode::CourseInactive
    )]
    pub course: Box<Account<'info, Course>>,
    #[account(
        init,
        payer = student,
        space = Enrollment::INIT_SPACE,
        seeds = [ENROLLMENT_SEED, &course.course_id.to_le_bytes(), student.key().as_ref()],
        bump
    )]
    pub enrollment: Box<Account<'info, Enrollment>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(exam_id: u64, _title: String, _question_count: u8, _encrypted_exam: EncryptedExamInput)]
pub struct CreateExam<'info> {
    #[account(mut)]
    pub tutor: Signer<'info>,
    #[account(
        seeds = [USER_SEED, tutor.key().as_ref()],
        bump = tutor_profile.bump
    )]
    pub tutor_profile: Account<'info, User>,
    #[account(
        mut,
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    #[account(
        seeds = [COURSE_SEED, &course.course_id.to_le_bytes()],
        bump = course.bump
    )]
    pub course: Account<'info, Course>,
    #[account(
        init,
        payer = tutor,
        space = Exam::INIT_SPACE,
        seeds = [EXAM_SEED, &exam_id.to_le_bytes()],
        bump
    )]
    pub exam: Box<Account<'info, Exam>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RequestExamAccess<'info> {
    #[account(mut)]
    pub student: Signer<'info>,
    #[account(
        seeds = [USER_SEED, student.key().as_ref()],
        bump = student_profile.bump
    )]
    pub student_profile: Box<Account<'info, User>>,
    #[account(
        seeds = [COURSE_SEED, &course.course_id.to_le_bytes()],
        bump = course.bump,
        constraint = course.active @ ErrorCode::CourseInactive
    )]
    pub course: Box<Account<'info, Course>>,
    #[account(
        seeds = [ENROLLMENT_SEED, &course.course_id.to_le_bytes(), student.key().as_ref()],
        bump = enrollment.bump,
        constraint = enrollment.course_id == course.course_id @ ErrorCode::NotEnrolled
    )]
    pub enrollment: Box<Account<'info, Enrollment>>,
    #[account(
        seeds = [EXAM_SEED, &exam.exam_id.to_le_bytes()],
        bump = exam.bump,
        constraint = exam.course_id == course.course_id @ ErrorCode::CourseExamMismatch,
        constraint = exam.active @ ErrorCode::ExamInactive
    )]
    pub exam: Box<Account<'info, Exam>>,
    #[account(
        init,
        payer = student,
        space = ExamAccess::INIT_SPACE,
        seeds = [EXAM_ACCESS_SEED, &exam.exam_id.to_le_bytes(), student.key().as_ref()],
        bump
    )]
    pub exam_access: Box<Account<'info, ExamAccess>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GrantExamAccess<'info> {
    #[account(mut)]
    pub tutor: Signer<'info>,
    #[account(
        seeds = [USER_SEED, tutor.key().as_ref()],
        bump = tutor_profile.bump
    )]
    pub tutor_profile: Box<Account<'info, User>>,
    #[account(
        seeds = [COURSE_SEED, &course.course_id.to_le_bytes()],
        bump = course.bump,
        constraint = course.active @ ErrorCode::CourseInactive
    )]
    pub course: Box<Account<'info, Course>>,
    #[account(
        seeds = [EXAM_SEED, &exam.exam_id.to_le_bytes()],
        bump = exam.bump,
        constraint = exam.course_id == course.course_id @ ErrorCode::CourseExamMismatch,
        constraint = exam.active @ ErrorCode::ExamInactive
    )]
    pub exam: Box<Account<'info, Exam>>,
    #[account(
        seeds = [ENROLLMENT_SEED, &course.course_id.to_le_bytes(), exam_access.student.as_ref()],
        bump = enrollment.bump,
        constraint = enrollment.course_id == course.course_id @ ErrorCode::NotEnrolled,
        constraint = enrollment.student == exam_access.student @ ErrorCode::NotEnrolled
    )]
    pub enrollment: Box<Account<'info, Enrollment>>,
    #[account(
        mut,
        seeds = [EXAM_ACCESS_SEED, &exam.exam_id.to_le_bytes(), exam_access.student.as_ref()],
        bump = exam_access.bump
    )]
    pub exam_access: Box<Account<'info, ExamAccess>>,
}

#[queue_computation_accounts("grade_exam", student)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct TakeExam<'info> {
    #[account(mut)]
    pub student: Signer<'info>,
    #[account(
        seeds = [USER_SEED, student.key().as_ref()],
        bump = student_profile.bump
    )]
    pub student_profile: Box<Account<'info, User>>,
    #[account(
        seeds = [COURSE_SEED, &course.course_id.to_le_bytes()],
        bump = course.bump,
        constraint = course.active @ ErrorCode::CourseInactive
    )]
    pub course: Box<Account<'info, Course>>,
    #[account(
        seeds = [ENROLLMENT_SEED, &course.course_id.to_le_bytes(), student.key().as_ref()],
        bump = enrollment.bump,
        constraint = enrollment.course_id == course.course_id @ ErrorCode::NotEnrolled
    )]
    pub enrollment: Box<Account<'info, Enrollment>>,
    #[account(
        seeds = [EXAM_SEED, &exam.exam_id.to_le_bytes()],
        bump = exam.bump,
        constraint = exam.course_id == course.course_id @ ErrorCode::CourseExamMismatch,
        constraint = exam.active @ ErrorCode::ExamInactive
    )]
    pub exam: Box<Account<'info, Exam>>,
    #[account(
        seeds = [EXAM_ACCESS_SEED, &exam.exam_id.to_le_bytes(), student.key().as_ref()],
        bump = exam_access.bump,
        constraint = exam_access.exam_id == exam.exam_id @ ErrorCode::ExamAccessMismatch,
        constraint = exam_access.student == student.key() @ ErrorCode::ExamAccessMismatch,
        constraint = exam_access.granted @ ErrorCode::ExamAccessNotGranted
    )]
    pub exam_access: Box<Account<'info, ExamAccess>>,
    #[account(
        init,
        payer = student,
        space = ExamSession::INIT_SPACE,
        seeds = [SESSION_SEED, &exam.exam_id.to_le_bytes(), student.key().as_ref()],
        bump
    )]
    pub session: Box<Account<'info, ExamSession>>,
    #[account(
        init_if_needed,
        space = 9,
        payer = student,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Box<Account<'info, ArciumSignerAccount>>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(
        mut,
        address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: mempool_account, checked by the Arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: executing_pool, checked by the Arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: computation_account, checked by the Arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_GRADE_EXAM))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Box<Account<'info, FeePool>>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Box<Account<'info, ClockAccount>>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("grade_exam")]
#[derive(Accounts)]
pub struct GradeExamCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_GRADE_EXAM))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account, checked by Arcium program via constraints.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint.
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [SESSION_SEED, &session.exam_id.to_le_bytes(), session.student.as_ref()],
        bump = session.bump
    )]
    pub session: Box<Account<'info, ExamSession>>,
}

#[init_computation_definition_accounts("grade_exam", payer)]
#[derive(Accounts)]
pub struct InitGradeExamCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by Arcium program.
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address_lookup_table, checked by Arcium program.
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut_program is the Address Lookup Table program.
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct GlobalConfig {
    pub authority: Pubkey,
    pub course_counter: u64,
    pub exam_counter: u64,
    pub bump: u8,
}

impl GlobalConfig {
    pub const INIT_SPACE: usize = 8 + 32 + 8 + 8 + 1;
}

#[account]
pub struct User {
    pub authority: Pubkey,
    pub name: String,
    pub role: Role,
    pub bump: u8,
}

impl User {
    pub const INIT_SPACE: usize = 8 + 32 + string_space(MAX_NAME_LEN) + 1 + 1;
}

#[account]
pub struct Course {
    pub course_id: u64,
    pub title: String,
    pub tutor: Pubkey,
    pub tutor_name: String,
    pub active: bool,
    pub bump: u8,
}

impl Course {
    pub const INIT_SPACE: usize =
        8 + 8 + string_space(MAX_COURSE_TITLE_LEN) + 32 + string_space(MAX_NAME_LEN) + 1 + 1;
}

#[account]
pub struct Enrollment {
    pub course_id: u64,
    pub student: Pubkey,
    pub enrolled_at: i64,
    pub bump: u8,
}

impl Enrollment {
    pub const INIT_SPACE: usize = 8 + 8 + 32 + 8 + 1;
}

#[account]
pub struct Exam {
    pub exam_id: u64,
    pub course_id: u64,
    pub tutor: Pubkey,
    pub title: String,
    pub question_count: u8,
    pub active: bool,
    pub content_pubkey: [u8; 32],
    pub content_nonce: u128,
    pub content_ciphertexts: Vec<[u8; 32]>,
    pub answer_key_nonce: u128,
    pub answer_key_ciphertexts: Vec<[u8; 32]>,
    pub bump: u8,
}

impl Exam {
    pub const INIT_SPACE: usize = 8
        + 8
        + 8
        + 32
        + string_space(MAX_EXAM_TITLE_LEN)
        + 1
        + 1
        + 32
        + 16
        + vec_space(32, MAX_EXAM_CONTENT_CIPHERTEXTS)
        + 16
        + vec_space(32, MAX_QUESTIONS_PER_EXAM)
        + 1;
}

#[account]
pub struct ExamAccess {
    pub exam_id: u64,
    pub course_id: u64,
    pub student: Pubkey,
    pub student_content_pubkey: [u8; 32],
    pub content_key_nonce: u128,
    pub encrypted_content_key: Vec<[u8; 32]>,
    pub granted: bool,
    pub bump: u8,
}

impl ExamAccess {
    pub const INIT_SPACE: usize =
        8 + 8 + 8 + 32 + 32 + 16 + vec_space(32, MAX_CONTENT_KEY_CIPHERTEXTS) + 1 + 1;
}

#[account]
pub struct ExamSession {
    pub exam_id: u64,
    pub course_id: u64,
    pub student: Pubkey,
    pub answers: Vec<u8>,
    pub score: u16,
    pub correctness_mask: u32,
    pub correctness: Vec<bool>,
    pub completed: bool,
    pub completed_at: i64,
    pub bump: u8,
}

impl ExamSession {
    pub const INIT_SPACE: usize = 8
        + 8
        + 8
        + 32
        + vec_space(1, MAX_QUESTIONS_PER_EXAM)
        + 2
        + 4
        + vec_space(1, MAX_QUESTIONS_PER_EXAM)
        + 1
        + 8
        + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Tutor,
    Student,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct EncryptedExamInput {
    pub content_pubkey: [u8; 32],
    pub content_nonce: u128,
    pub content_ciphertexts: Vec<[u8; 32]>,
    pub answer_key_nonce: u128,
    pub answer_key_ciphertexts: Vec<[u8; 32]>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct EncryptedContentKeyInput {
    pub nonce: u128,
    pub ciphertexts: Vec<[u8; 32]>,
}

#[event]
pub struct UserRegistered {
    pub authority: Pubkey,
    pub name: String,
    pub role: Role,
}

#[event]
pub struct CourseCreated {
    pub course_id: u64,
    pub title: String,
    pub tutor: Pubkey,
}

#[event]
pub struct StudentEnrolled {
    pub course_id: u64,
    pub student: Pubkey,
}

#[event]
pub struct ExamCreated {
    pub exam_id: u64,
    pub course_id: u64,
    pub tutor: Pubkey,
    pub question_count: u8,
}

#[event]
pub struct ExamAccessRequested {
    pub exam_id: u64,
    pub course_id: u64,
    pub student: Pubkey,
}

#[event]
pub struct ExamAccessGranted {
    pub exam_id: u64,
    pub course_id: u64,
    pub student: Pubkey,
}

#[event]
pub struct ExamSubmitted {
    pub exam_id: u64,
    pub course_id: u64,
    pub student: Pubkey,
}

#[event]
pub struct ExamCompleted {
    pub exam_id: u64,
    pub course_id: u64,
    pub student: Pubkey,
    pub score: u16,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Only tutors can perform this action.")]
    TutorOnly,
    #[msg("Only students can perform this action.")]
    StudentOnly,
    #[msg("The provided name exceeds the maximum length.")]
    NameTooLong,
    #[msg("The provided course title exceeds the maximum length.")]
    CourseTitleTooLong,
    #[msg("The provided exam title exceeds the maximum length.")]
    ExamTitleTooLong,
    #[msg("Course IDs must match the next global counter value.")]
    InvalidCourseId,
    #[msg("Exam IDs must match the next global counter value.")]
    InvalidExamId,
    #[msg("Only the tutor who owns the course can manage its exams.")]
    UnauthorizedTutor,
    #[msg("The course is inactive.")]
    CourseInactive,
    #[msg("The exam is inactive.")]
    ExamInactive,
    #[msg("The student is not enrolled in this course.")]
    NotEnrolled,
    #[msg("The exam does not belong to the provided course.")]
    CourseExamMismatch,
    #[msg("The number of questions is outside the supported range.")]
    QuestionCountOutOfRange,
    #[msg("The encrypted exam content is outside the supported range.")]
    EncryptedExamContentOutOfRange,
    #[msg("The encrypted answer key length must match the question count.")]
    InvalidAnswerKeyLength,
    #[msg("The encrypted content key is outside the supported range.")]
    EncryptedContentKeyOutOfRange,
    #[msg("The exam access account does not match the exam or student.")]
    ExamAccessMismatch,
    #[msg("Exam access has not been granted for this student.")]
    ExamAccessNotGranted,
    #[msg("The submitted answer count does not match the exam.")]
    InvalidAnswerCount,
    #[msg("Each submitted answer must be between 0 and 3.")]
    AnswerOutOfRange,
    #[msg("Math overflow occurred.")]
    MathOverflow,
    #[msg("The Arcium computation was aborted.")]
    AbortedComputation,
    #[msg("Arcium cluster is not set.")]
    ClusterNotSet,
}

fn validate_string_len(value: &str, max_len: usize, error: ErrorCode) -> Result<()> {
    if value.len() > max_len {
        return Err(error.into());
    }
    Ok(())
}

const fn string_space(max_len: usize) -> usize {
    4 + max_len
}

const fn vec_space(element_space: usize, max_items: usize) -> usize {
    4 + (element_space * max_items)
}
