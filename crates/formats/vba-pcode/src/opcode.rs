//! The canonical opcode table (`pcode-opcodes.md` section 4).
//!
//! Generated from the table of the repository document; names are the
//! document's labels in Rust casing. The operand shape of each opcode is the
//! classification of section 5 of the document.

/// How the operands of an instruction are laid out after the instruction
/// word (section 5 of the opcode document).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    /// No operands.
    None,
    /// One identifier reference.
    Name,
    /// Identifier reference, then a word (argument count).
    NameW,
    /// One plain word.
    W,
    /// Two plain words.
    WW,
    /// Four plain words.
    WWWW,
    /// One object-table offset (word).
    Obj,
    /// One indirect-table offset (dword).
    Rec,
    /// Indirect-table offset (dword), plus one word when `MOD & 0x10`.
    VarDecl,
    /// Procedure context: one dword on x86, two on x64.
    Ctx,
    /// Identifier reference, word, indirect-table offset (dword).
    NameWRec,
    /// Variable-length trailer only.
    V,
    /// One word, then a variable-length trailer.
    WV,
}

macro_rules! opcodes {
    ($(($num:expr, $variant:ident, $label:expr, $shape:ident),)*) => {
        /// A canonical opcode. Names follow the labels of the opcode
        /// document; semantics beyond the operand layout are inferred there.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u16)]
        #[allow(missing_docs)]
        pub enum Opcode {
            $($variant = $num,)*
        }

        impl Opcode {
            /// Every opcode in canonical order.
            pub const ALL: &'static [Opcode] = &[$(Opcode::$variant,)*];

            /// Looks up a canonical number (0 to 263).
            pub fn from_canonical(n: u16) -> Option<Opcode> {
                Self::ALL.get(usize::from(n)).copied().filter(|o| o.canonical() == n)
            }

            /// The canonical number.
            pub fn canonical(self) -> u16 {
                self as u16
            }

            /// The label used by the opcode document, for example
            /// `READ_SIMPLE`.
            pub fn label(self) -> &'static str {
                match self {
                    $(Opcode::$variant => $label,)*
                }
            }

            /// The operand layout.
            pub fn shape(self) -> Shape {
                match self {
                    $(Opcode::$variant => Shape::$shape,)*
                }
            }
        }
    };
}

opcodes! {
    (0, BoolImplication, "BOOL_IMPLICATION", None),
    (1, BoolEquivalence, "BOOL_EQUIVALENCE", None),
    (2, BoolExclusiveOr, "BOOL_EXCLUSIVE_OR", None),
    (3, BoolDisjunction, "BOOL_DISJUNCTION", None),
    (4, BoolConjunction, "BOOL_CONJUNCTION", None),
    (5, CompareEqual, "COMPARE_EQUAL", None),
    (6, CompareNotEqual, "COMPARE_NOT_EQUAL", None),
    (7, CompareLessEqual, "COMPARE_LESS_EQUAL", None),
    (8, CompareGreaterEqual, "COMPARE_GREATER_EQUAL", None),
    (9, CompareLess, "COMPARE_LESS", None),
    (10, CompareGreater, "COMPARE_GREATER", None),
    (11, NumAdd, "NUM_ADD", None),
    (12, NumSubtract, "NUM_SUBTRACT", None),
    (13, NumRemainder, "NUM_REMAINDER", None),
    (14, NumIntDivide, "NUM_INT_DIVIDE", None),
    (15, NumMultiply, "NUM_MULTIPLY", None),
    (16, NumDivide, "NUM_DIVIDE", None),
    (17, TextJoin, "TEXT_JOIN", None),
    (18, ComparePattern, "COMPARE_PATTERN", None),
    (19, NumPower, "NUM_POWER", None),
    (20, CompareObjectIdentity, "COMPARE_OBJECT_IDENTITY", None),
    (21, BoolNegate, "BOOL_NEGATE", None),
    (22, NumNegate, "NUM_NEGATE", None),
    (23, IntrinsicAbsoluteValue, "INTRINSIC_ABSOLUTE_VALUE", None),
    (24, IntrinsicTruncate, "INTRINSIC_TRUNCATE", None),
    (25, IntrinsicFloor, "INTRINSIC_FLOOR", None),
    (26, IntrinsicSign, "INTRINSIC_SIGN", None),
    (27, IntrinsicLengthChars, "INTRINSIC_LENGTH_CHARS", None),
    (28, IntrinsicLengthBytes, "INTRINSIC_LENGTH_BYTES", None),
    (29, ExprParentheses, "EXPR_PARENTHESES", None),
    (30, ExprFileNumberMark, "EXPR_FILE_NUMBER_MARK", None),
    (31, ReadTargetName, "READ_TARGET_NAME", Name),
    (32, ReadSimple, "READ_SIMPLE", Name),
    (33, ReadDot, "READ_DOT", Name),
    (34, ReadBang, "READ_BANG", Name),
    (35, ReadDefaultIndexed, "READ_DEFAULT_INDEXED", W),
    (36, ReadSimpleParams, "READ_SIMPLE_PARAMS", NameW),
    (37, ReadDotParams, "READ_DOT_PARAMS", NameW),
    (38, ReadBangParams, "READ_BANG_PARAMS", NameW),
    (39, WriteSimple, "WRITE_SIMPLE", Name),
    (40, WriteDot, "WRITE_DOT", Name),
    (41, WriteBang, "WRITE_BANG", Name),
    (42, WriteDefaultIndexed, "WRITE_DEFAULT_INDEXED", W),
    (43, WriteSimpleParams, "WRITE_SIMPLE_PARAMS", NameW),
    (44, WriteDotParams, "WRITE_DOT_PARAMS", NameW),
    (45, WriteBangParams, "WRITE_BANG_PARAMS", NameW),
    (46, WriteRefSimple, "WRITE_REF_SIMPLE", Name),
    (47, WriteRefDot, "WRITE_REF_DOT", Name),
    (48, WriteRefBang, "WRITE_REF_BANG", Name),
    (49, WriteRefDefaultIndexed, "WRITE_REF_DEFAULT_INDEXED", W),
    (50, WriteRefSimpleParams, "WRITE_REF_SIMPLE_PARAMS", NameW),
    (51, WriteRefDotParams, "WRITE_REF_DOT_PARAMS", NameW),
    (52, WriteRefBangParams, "WRITE_REF_BANG_PARAMS", NameW),
    (53, ReadWithDot, "READ_WITH_DOT", Name),
    (54, ReadWithBang, "READ_WITH_BANG", Name),
    (55, ReadWithDotParams, "READ_WITH_DOT_PARAMS", NameW),
    (56, ReadWithBangParams, "READ_WITH_BANG_PARAMS", NameW),
    (57, WriteWithDot, "WRITE_WITH_DOT", Name),
    (58, WriteWithBang, "WRITE_WITH_BANG", Name),
    (59, WriteWithDotParams, "WRITE_WITH_DOT_PARAMS", NameW),
    (60, WriteWithBangParams, "WRITE_WITH_BANG_PARAMS", NameW),
    (61, WriteRefWithDot, "WRITE_REF_WITH_DOT", Name),
    (62, WriteRefWithBang, "WRITE_REF_WITH_BANG", Name),
    (63, WriteRefWithDotParams, "WRITE_REF_WITH_DOT_PARAMS", NameW),
    (64, WriteRefWithBangParams, "WRITE_REF_WITH_BANG_PARAMS", NameW),
    (65, InvokeSimple, "INVOKE_SIMPLE", NameW),
    (66, InvokeDot, "INVOKE_DOT", NameW),
    (67, InvokeWithDot, "INVOKE_WITH_DOT", NameW),
    (68, ArrayElementParams, "ARRAY_ELEMENT_PARAMS", NameW),
    (69, DebugAssert, "DEBUG_ASSERT", None),
    (70, StmtBegin, "STMT_BEGIN", W),
    (71, StmtBeginImplicit, "STMT_BEGIN_IMPLICIT", None),
    (72, LineBegin, "LINE_BEGIN", None),
    (73, AddressOfSimple, "ADDRESS_OF_SIMPLE", Name),
    (74, AddressOfDot, "ADDRESS_OF_DOT", Name),
    (75, CaseValue, "CASE_VALUE", None),
    (76, CaseRange, "CASE_RANGE", None),
    (77, CaseIsGreater, "CASE_IS_GREATER", None),
    (78, CaseIsLess, "CASE_IS_LESS", None),
    (79, CaseIsGreaterEqual, "CASE_IS_GREATER_EQUAL", None),
    (80, CaseIsLessEqual, "CASE_IS_LESS_EQUAL", None),
    (81, CaseIsNotEqual, "CASE_IS_NOT_EQUAL", None),
    (82, CaseIsEqual, "CASE_IS_EQUAL", None),
    (83, CaseDefault, "CASE_DEFAULT", None),
    (84, CaseListEnd, "CASE_LIST_END", None),
    (85, DrawCircle, "DRAW_CIRCLE", W),
    (86, FileClose, "FILE_CLOSE", W),
    (87, FileCloseAll, "FILE_CLOSE_ALL", None),
    (88, ConvertToType, "CONVERT_TO_TYPE", None),
    (89, ConvertToTypeVariant, "CONVERT_TO_TYPE_VARIANT", None),
    (90, ProcContext, "PROC_CONTEXT", Ctx),
    (91, DebugPrefix, "DEBUG_PREFIX", None),
    (92, DeftypeRange, "DEFTYPE_RANGE", WW),
    (93, DimMarker, "DIM_MARKER", None),
    (94, DimMarkerImplicit, "DIM_MARKER_IMPLICIT", None),
    (95, DoOpen, "DO_OPEN", None),
    (96, YieldEvents, "YIELD_EVENTS", None),
    (97, DoOpenUntil, "DO_OPEN_UNTIL", None),
    (98, DoOpenWhile, "DO_OPEN_WHILE", None),
    (99, ElseSingleLine, "ELSE_SINGLE_LINE", None),
    (100, BlockElseBranch, "BLOCK_ELSE_BRANCH", None),
    (101, BlockElseifBranch, "BLOCK_ELSEIF_BRANCH", None),
    (102, BlockElseifBranchTypeTest, "BLOCK_ELSEIF_BRANCH_TYPE_TEST", Obj),
    (103, HaltEnd, "HALT_END", None),
    (104, ProcContextEnd, "PROC_CONTEXT_END", None),
    (105, ProcEndFunction, "PROC_END_FUNCTION", None),
    (106, EndifSingleLine, "ENDIF_SINGLE_LINE", None),
    (107, BlockIfClose, "BLOCK_IF_CLOSE", None),
    (108, ImmediateEnd, "IMMEDIATE_END", None),
    (109, ProcEndProperty, "PROC_END_PROPERTY", None),
    (110, SwitchEnd, "SWITCH_END", None),
    (111, ProcEndSub, "PROC_END_SUB", None),
    (112, TypeDeclEnd, "TYPE_DECL_END", None),
    (113, WithClose, "WITH_CLOSE", None),
    (114, EraseArrays, "ERASE_ARRAYS", W),
    (115, ErrRaiseStmt, "ERR_RAISE_STMT", None),
    (116, DeclareEvent, "DECLARE_EVENT", Rec),
    (117, EventFireSimple, "EVENT_FIRE_SIMPLE", NameW),
    (118, EventFireDot, "EVENT_FIRE_DOT", NameW),
    (119, EventFireWithDot, "EVENT_FIRE_WITH_DOT", NameW),
    (120, LeaveDo, "LEAVE_DO", None),
    (121, LeaveFor, "LEAVE_FOR", None),
    (122, LeaveFunction, "LEAVE_FUNCTION", None),
    (123, LeaveProperty, "LEAVE_PROPERTY", None),
    (124, LeaveSub, "LEAVE_SUB", None),
    (125, IntrinsicCurrentDir, "INTRINSIC_CURRENT_DIR", None),
    (126, IntrinsicDirListing, "INTRINSIC_DIR_LISTING", None),
    (127, ReservedA, "RESERVED_A", None),
    (128, ReservedB, "RESERVED_B", None),
    (129, IntrinsicErrorText, "INTRINSIC_ERROR_TEXT", None),
    (130, IntrinsicFormatValue, "INTRINSIC_FORMAT_VALUE", None),
    (131, IntrinsicNextFileNumber, "INTRINSIC_NEXT_FILE_NUMBER", None),
    (132, IntrinsicSearchA, "INTRINSIC_SEARCH_A", None),
    (133, IntrinsicSearchB, "INTRINSIC_SEARCH_B", None),
    (134, IntrinsicSearchC, "INTRINSIC_SEARCH_C", None),
    (135, IntrinsicSearchBytesA, "INTRINSIC_SEARCH_BYTES_A", None),
    (136, IntrinsicSearchBytesB, "INTRINSIC_SEARCH_BYTES_B", None),
    (137, IntrinsicSearchBytesC, "INTRINSIC_SEARCH_BYTES_C", None),
    (138, IntrinsicLowerBound, "INTRINSIC_LOWER_BOUND", W),
    (139, IntrinsicSubstring, "INTRINSIC_SUBSTRING", None),
    (140, IntrinsicSubstringBytes, "INTRINSIC_SUBSTRING_BYTES", None),
    (141, IntrinsicTextCompareA, "INTRINSIC_TEXT_COMPARE_A", None),
    (142, IntrinsicTextCompareB, "INTRINSIC_TEXT_COMPARE_B", None),
    (143, IntrinsicRepeatFromCode, "INTRINSIC_REPEAT_FROM_CODE", None),
    (144, IntrinsicRepeatFromText, "INTRINSIC_REPEAT_FROM_TEXT", None),
    (145, IntrinsicUpperBound, "INTRINSIC_UPPER_BOUND", W),
    (146, ForCounted, "FOR_COUNTED", None),
    (147, ForCollection, "FOR_COLLECTION", None),
    (148, ForCollectionTyped, "FOR_COLLECTION_TYPED", Obj),
    (149, ForCountedStep, "FOR_COUNTED_STEP", None),
    (150, ProcHeader, "PROC_HEADER", Rec),
    (151, ProcHeaderAlt, "PROC_HEADER_ALT", Rec),
    (152, FileReadRecord, "FILE_READ_RECORD", None),
    (153, BranchSubroutine, "BRANCH_SUBROUTINE", Name),
    (154, BranchGoto, "BRANCH_GOTO", Name),
    (155, IfSingleLine, "IF_SINGLE_LINE", None),
    (156, IfBlockOpen, "IF_BLOCK_OPEN", None),
    (157, TypeTest, "TYPE_TEST", Obj),
    (158, IfBlockOpenTypeTest, "IF_BLOCK_OPEN_TYPE_TEST", Obj),
    (159, ImplementsStmt, "IMPLEMENTS_STMT", WW),
    (160, ReadListOpen, "READ_LIST_OPEN", None),
    (161, ReadListClose, "READ_LIST_CLOSE", None),
    (162, ReadListItem, "READ_LIST_ITEM", None),
    (163, LabelNamed, "LABEL_NAMED", Name),
    (164, LetKeywordMarker, "LET_KEYWORD_MARKER", None),
    (165, DrawLine, "DRAW_LINE", W),
    (166, LineContinuation, "LINE_CONTINUATION", V),
    (167, FileReadLine, "FILE_READ_LINE", None),
    (168, LabelNumeric, "LABEL_NUMERIC", Name),
    (169, ConstCurrency, "CONST_CURRENCY", WWWW),
    (170, ConstDate, "CONST_DATE", WWWW),
    (171, ArgDefaultPlaceholder, "ARG_DEFAULT_PLACEHOLDER", None),
    (172, ConstInt16Decimal, "CONST_INT16_DECIMAL", W),
    (173, ConstInt32Decimal, "CONST_INT32_DECIMAL", WW),
    (174, ConstInt64Decimal, "CONST_INT64_DECIMAL", WWWW),
    (175, ConstInt16Hex, "CONST_INT16_HEX", W),
    (176, ConstInt32Hex, "CONST_INT32_HEX", WW),
    (177, ConstInt64Hex, "CONST_INT64_HEX", WWWW),
    (178, ConstNullObject, "CONST_NULL_OBJECT", None),
    (179, ConstInt16Octal, "CONST_INT16_OCTAL", W),
    (180, ConstInt32Octal, "CONST_INT32_OCTAL", WW),
    (181, ConstInt64Octal, "CONST_INT64_OCTAL", WWWW),
    (182, ConstFloat32, "CONST_FLOAT32", WW),
    (183, ConstFloat64, "CONST_FLOAT64", WWWW),
    (184, ConstSmallInt, "CONST_SMALL_INT", None),
    (185, ConstText, "CONST_TEXT", V),
    (186, ConstKeywordValue, "CONST_KEYWORD_VALUE", None),
    (187, FileLock, "FILE_LOCK", None),
    (188, DoClose, "DO_CLOSE", None),
    (189, DoCloseUntil, "DO_CLOSE_UNTIL", None),
    (190, DoCloseWhile, "DO_CLOSE_WHILE", None),
    (191, LeftAlignAssign, "LEFT_ALIGN_ASSIGN", None),
    (192, SelfRef, "SELF_REF", None),
    (193, SelfRefImplicit, "SELF_REF_IMPLICIT", None),
    (194, RedimDot, "REDIM_DOT", NameWRec),
    (195, RedimDotWith, "REDIM_DOT_WITH", NameWRec),
    (196, RedimDotTyped, "REDIM_DOT_TYPED", NameWRec),
    (197, RedimDotTypedWith, "REDIM_DOT_TYPED_WITH", NameWRec),
    (198, SubstrAssign, "SUBSTR_ASSIGN", None),
    (199, SubstrBytesAssign, "SUBSTR_BYTES_ASSIGN", None),
    (200, RenameFile, "RENAME_FILE", None),
    (201, NewObject, "NEW_OBJECT", Obj),
    (202, ForAdvance, "FOR_ADVANCE", None),
    (203, ForAdvanceNamed, "FOR_ADVANCE_NAMED", None),
    (204, ErrHandlerSet, "ERR_HANDLER_SET", Name),
    (205, BranchComputedSubroutine, "BRANCH_COMPUTED_SUBROUTINE", V),
    (206, BranchComputedGoto, "BRANCH_COMPUTED_GOTO", V),
    (207, FileOpen, "FILE_OPEN", W),
    (208, OptionStmt, "OPTION_STMT", None),
    (209, OptionBaseMarker, "OPTION_BASE_MARKER", None),
    (210, ArgForceByval, "ARG_FORCE_BYVAL", None),
    (211, ArgMissing, "ARG_MISSING", None),
    (212, ArgNamed, "ARG_NAMED", Name),
    (213, OutChannel, "OUT_CHANNEL", None),
    (214, OutComma, "OUT_COMMA", None),
    (215, OutStatementEnd, "OUT_STATEMENT_END", None),
    (216, OutItemThenComma, "OUT_ITEM_THEN_COMMA", None),
    (217, OutItemLast, "OUT_ITEM_LAST", None),
    (218, OutItemThenSemicolon, "OUT_ITEM_THEN_SEMICOLON", None),
    (219, OutNewline, "OUT_NEWLINE", None),
    (220, OutTargetObject, "OUT_TARGET_OBJECT", None),
    (221, OutSemicolon, "OUT_SEMICOLON", None),
    (222, OutSpaces, "OUT_SPACES", None),
    (223, OutTab, "OUT_TAB", None),
    (224, OutTabThenComma, "OUT_TAB_THEN_COMMA", None),
    (225, DrawPoint, "DRAW_POINT", W),
    (226, FileWriteRecord, "FILE_WRITE_RECORD", None),
    (227, CommentQuote, "COMMENT_QUOTE", WV),
    (228, RedimSimple, "REDIM_SIMPLE", NameWRec),
    (229, RedimSimpleTyped, "REDIM_SIMPLE_TYPED", NameWRec),
    (230, TextUnparsed, "TEXT_UNPARSED", V),
    (231, CommentRemKeyword, "COMMENT_REM_KEYWORD", V),
    (232, ErrResume, "ERR_RESUME", Name),
    (233, BranchReturn, "BRANCH_RETURN", None),
    (234, RightAlignAssign, "RIGHT_ALIGN_ASSIGN", None),
    (235, DrawScale, "DRAW_SCALE", W),
    (236, FileSeek, "FILE_SEEK", None),
    (237, SwitchBegin, "SWITCH_BEGIN", None),
    (238, SwitchCaseClass, "SWITCH_CASE_CLASS", Obj),
    (239, SwitchBeginType, "SWITCH_BEGIN_TYPE", None),
    (240, SetKeywordMarker, "SET_KEYWORD_MARKER", None),
    (241, FrameReserve, "FRAME_RESERVE", WW),
    (242, HaltStop, "HALT_STOP", None),
    (243, TypeDecl, "TYPE_DECL", Rec),
    (244, FileUnlock, "FILE_UNLOCK", None),
    (245, VarDecl, "VAR_DECL", VarDecl),
    (246, WhileClose, "WHILE_CLOSE", None),
    (247, WhileOpen, "WHILE_OPEN", None),
    (248, WithOpen, "WITH_OPEN", None),
    (249, FileWriteChannel, "FILE_WRITE_CHANNEL", None),
    (250, ConstExprWithCall, "CONST_EXPR_WITH_CALL", None),
    (251, PpDefine, "PP_DEFINE", Name),
    (252, PpIf, "PP_IF", None),
    (253, PpElse, "PP_ELSE", None),
    (254, PpElseif, "PP_ELSEIF", None),
    (255, PpEndif, "PP_ENDIF", None),
    (256, PpMarker, "PP_MARKER", None),
    (257, ForVarExprClose, "FOR_VAR_EXPR_CLOSE", None),
    (258, ForVarExprOpen, "FOR_VAR_EXPR_OPEN", None),
    (259, RedimNewMarker, "REDIM_NEW_MARKER", None),
    (260, WithExprOpen, "WITH_EXPR_OPEN", None),
    (261, WriteSimpleAmbiguous, "WRITE_SIMPLE_AMBIGUOUS", Name),
    (262, EnumDeclEnd, "ENUM_DECL_END", None),
    (263, InvalidSentinel, "INVALID_SENTINEL", None),
}

/// How well the meaning of an opcode is established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Evidence {
    /// Operand shape, operand order and stack effect were confirmed by
    /// differential comparison with the source text of the same statement.
    Verified,
    /// Occurs in the observed material, but its meaning rests on the
    /// opcode document or on partial agreement only.
    Inferred,
    /// Never observed; the operand shape comes from the document alone.
    Unseen,
}

/// Opcodes whose meaning was confirmed by differential comparison on the
/// observed material (VBA 7.0, x86).
///
/// Evidence: the cache instructions of a statement are replayed on an
/// expression stack under the documented stack semantics and the resulting
/// expression is compared with the source expression parsed with the VBAL
/// operator precedence. Statements compared: 1,098 assignments (1,097
/// agree), 237 block `If` conditions (237), 28 single-line `If` conditions
/// (27; the other has a `StrComp` the check does not model), 11 `ElseIf` (11), 18 `Do`/`Loop`
/// conditions (18), 2 `Select Case` subjects (2), 35 `For` headers with
/// start, end and step in order (35), 317 `Case` lists with values, ranges
/// and `Is` comparisons (317), 15 `Debug.Print` lists with separators (15)
/// and 137 call statements with argument count and `Call` keyword (137).
/// Marker opcodes without operands are confirmed by agreement between the
/// line they occur on and the keyword that starts the source line (zero
/// disagreements for each, apart from single-line `If` statements that
/// contain `Exit` or `GoTo`, and `Dim`-like declarations that share the
/// marker). The one disagreement left is `&HFFFF` stored as a 32-bit hex
/// literal with value 65535 where the language gives `-1`.
const VERIFIED: &[u16] = &[
    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 20, 21, 22, 23, 25, 26, 27, 29, 32,
    36, 39, 65, 75, 76, 81, 82, 83, 84, 91, 93, 95, 97, 98, 100, 101, 103, 105, 106, 107, 109, 110,
    111, 113, 121, 122, 124, 132, 133, 134, 138, 145, 146, 149, 150, 154, 155, 156, 172, 173, 176,
    178, 183, 185, 186, 188, 189, 190, 202, 203, 208, 216, 217, 218, 220, 237, 245, 246, 247, 257,
    258,
];

/// Opcodes that occur in the observed material (167 of 264): 135 on VBA 7.0
/// x86, 32 more in VBA 6 and 7 projects of both bitnesses, each decoding to
/// the exact length of every line it occurs on.
const SEEN: &[u16] = &[
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 25, 26, 27, 29,
    30, 32, 33, 34, 35, 36, 37, 39, 40, 41, 43, 44, 46, 47, 48, 50, 51, 53, 55, 57, 59, 61, 65, 66,
    67, 68, 69, 70, 71, 73, 75, 76, 77, 81, 82, 83, 84, 86, 88, 91, 93, 94, 95, 97, 98, 99, 100,
    101, 103, 105, 106, 107, 109, 110, 111, 112, 113, 114, 116, 117, 120, 121, 122, 123, 124, 132,
    133, 134, 138, 141, 142, 145, 146, 147, 149, 150, 152, 154, 155, 156, 157, 159, 163, 165, 166,
    167, 168, 170, 171, 172, 173, 175, 176, 178, 183, 185, 186, 188, 189, 190, 191, 194, 195, 198,
    201, 202, 203, 204, 207, 208, 209, 210, 211, 212, 213, 216, 217, 218, 220, 227, 228, 229, 230,
    232, 237, 240, 243, 245, 246, 247, 248, 250, 251, 252, 253, 255, 256, 257, 258, 260, 262,
];

impl Opcode {
    /// How well the meaning of this opcode is established.
    pub fn evidence(self) -> Evidence {
        let n = self.canonical();
        if VERIFIED.contains(&n) {
            Evidence::Verified
        } else if SEEN.contains(&n) {
            Evidence::Inferred
        } else {
            Evidence::Unseen
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_tables_are_sorted_and_consistent() {
        assert!(VERIFIED.windows(2).all(|w| w[0] < w[1]));
        assert!(SEEN.windows(2).all(|w| w[0] < w[1]));
        assert!(VERIFIED.iter().all(|v| SEEN.contains(v)));
        assert_eq!(SEEN.len(), 167);
        assert!(SEEN.iter().all(|&n| Opcode::from_canonical(n).is_some()));
        assert_eq!(Opcode::NumAdd.evidence(), Evidence::Verified);
        assert_eq!(Opcode::ReadDot.evidence(), Evidence::Inferred);
        assert_eq!(Opcode::FileOpen.evidence(), Evidence::Inferred);
        assert_eq!(Opcode::ImplementsStmt.evidence(), Evidence::Inferred);
        assert_eq!(Opcode::ProcContext.evidence(), Evidence::Unseen);
    }

    #[test]
    fn table_is_dense_and_consistent() {
        assert_eq!(Opcode::ALL.len(), 264);
        for (i, o) in Opcode::ALL.iter().enumerate() {
            assert_eq!(usize::from(o.canonical()), i);
            assert_eq!(Opcode::from_canonical(i as u16), Some(*o));
        }
        assert_eq!(Opcode::from_canonical(264), None);
        assert_eq!(Opcode::from_canonical(u16::MAX), None);
    }

    #[test]
    fn spot_checks() {
        assert_eq!(Opcode::from_canonical(32), Some(Opcode::ReadSimple));
        assert_eq!(Opcode::ReadSimple.label(), "READ_SIMPLE");
        assert_eq!(Opcode::ReadSimple.shape(), Shape::Name);
        assert_eq!(Opcode::from_canonical(245), Some(Opcode::VarDecl));
        assert_eq!(Opcode::VarDecl.shape(), Shape::VarDecl);
        assert_eq!(Opcode::from_canonical(150), Some(Opcode::ProcHeader));
        assert_eq!(Opcode::from_canonical(227), Some(Opcode::CommentQuote));
        assert_eq!(Opcode::CommentQuote.shape(), Shape::WV);
        assert_eq!(Opcode::from_canonical(208), Some(Opcode::OptionStmt));
        assert_eq!(Opcode::from_canonical(263), Some(Opcode::InvalidSentinel));
    }
}
