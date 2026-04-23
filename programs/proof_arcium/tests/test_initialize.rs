use proof_arcium::{
    Course, Enrollment, Exam, ExamAccess, ExamSession, GlobalConfig, User, MAX_QUESTIONS_PER_EXAM,
};

#[test]
fn account_sizes_fit_expected_limits() {
    assert_eq!(GlobalConfig::INIT_SPACE, 57);
    assert_eq!(User::INIT_SPACE, 110);
    assert_eq!(Course::INIT_SPACE, 222);
    assert_eq!(Enrollment::INIT_SPACE, 57);
    assert_eq!(Exam::INIT_SPACE, 8939);
    assert_eq!(ExamAccess::INIT_SPACE, 366);
    assert_eq!(ExamSession::INIT_SPACE, 112);
    assert!(Exam::INIT_SPACE < 10_240);
    assert!(ExamAccess::INIT_SPACE < 10_240);
    assert!(ExamSession::INIT_SPACE < 10_240);
    assert_eq!(MAX_QUESTIONS_PER_EXAM, 16);
}
