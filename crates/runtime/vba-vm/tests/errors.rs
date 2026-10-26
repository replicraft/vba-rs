//! End-to-end programs: `On Error`, `Resume`, `Err` and error propagation.

mod common;

use common::*;

#[test]
fn error_handling() {
    check(&[
        case(
            "on_error_goto_handler",
            "Sub Main()\nOn Error GoTo h\nDim x As Long\nx = 1 / 0\nDebug.Print \"not reached\"\nExit Sub\nh:\nDebug.Print \"err \" & Err.Number & \": \" & Err.Description\nEnd Sub\n",
            "err 11: Division by zero\n",
        ),
        case(
            "resume_next",
            "Sub Main()\nOn Error Resume Next\nDim x As Long\nx = 1 / 0\nDebug.Print \"after \" & Err.Number\nEnd Sub\n",
            "after 11\n",
        ),
        case(
            "resume_next_skips_statement_only",
            "Sub Main()\nOn Error Resume Next\nDim x As Long, y As Long\nx = 5: y = 1 / 0: x = x + 1\nDebug.Print x & \",\" & y\nEnd Sub\n",
            "6,0\n",
        ),
        case(
            "resume_retries",
            "Sub Main()\nDim d As Long, tries As Long\nd = 0\nOn Error GoTo repair\nDebug.Print 10 \\ d\nExit Sub\nrepair:\ntries = tries + 1\nd = 5\nResume\nEnd Sub\n",
            " 2 \n",
        ),
        case(
            "resume_label",
            "Sub Main()\nOn Error GoTo h\nError 5\nDebug.Print \"skipped\"\ndone:\nDebug.Print \"done\"\nExit Sub\nh:\nDebug.Print \"h\"\nResume done\nEnd Sub\n",
            "h\ndone\n",
        ),
        case(
            "err_raise_custom",
            "Sub Main()\nOn Error GoTo h\nErr.Raise 1234, \"Src\", \"custom text\"\nExit Sub\nh:\nDebug.Print Err.Number & \"|\" & Err.Source & \"|\" & Err.Description\nEnd Sub\n",
            "1234|Src|custom text\n",
        ),
        case(
            "err_raise_default_source",
            "Sub Main()\nOn Error Resume Next\nErr.Raise 9\nDebug.Print Err.Number & \"|\" & Err.Source & \"|\" & Err.Description\nEnd Sub\n",
            "9|VBAProject|Subscript out of range\n",
        ),
        case(
            "err_clear",
            "Sub Main()\nOn Error Resume Next\nErr.Raise 6\nErr.Clear\nDebug.Print Err.Number & \"/\" & Err.Description & \"/\"\nEnd Sub\n",
            "0//\n",
        ),
        case(
            "error_in_called_procedure",
            "Sub Inner()\nDim a(1 To 2) As Long\na(5) = 1\nEnd Sub\nSub Main()\nOn Error GoTo h\nInner\nDebug.Print \"no\"\nExit Sub\nh:\nDebug.Print \"caught \" & Err.Number\nEnd Sub\n",
            "caught 9\n",
        ),
        case(
            "error_unwinds_two_frames",
            "Sub C()\nErr.Raise 77\nEnd Sub\nSub B()\nC\nEnd Sub\nSub Main()\nOn Error GoTo h\nB\nExit Sub\nh:\nDebug.Print \"main caught \" & Err.Number\nEnd Sub\n",
            "main caught 77\n",
        ),
        case(
            "handler_in_callee_handles_first",
            "Sub C()\nOn Error GoTo h\nErr.Raise 5\nExit Sub\nh:\nDebug.Print \"c caught\"\nEnd Sub\nSub Main()\nOn Error GoTo h\nC\nDebug.Print \"back\"\nExit Sub\nh:\nDebug.Print \"main caught\"\nEnd Sub\n",
            "c caught\nback\n",
        ),
        case(
            "error_in_handler_propagates",
            "Sub C()\nOn Error GoTo h\nErr.Raise 5\nExit Sub\nh:\nErr.Raise 6\nEnd Sub\nSub Main()\nOn Error GoTo h\nC\nExit Sub\nh:\nDebug.Print \"main \" & Err.Number\nEnd Sub\n",
            "main 6\n",
        ),
        case(
            "on_error_goto_zero_disables",
            "Sub C()\nOn Error GoTo 0\nErr.Raise 5\nEnd Sub\nSub Main()\nOn Error GoTo h\nC\nExit Sub\nh:\nDebug.Print \"main \" & Err.Number\nEnd Sub\n",
            "main 5\n",
        ),
        case(
            "rearm_handler_with_goto_minus_one",
            "Sub Main()\nOn Error GoTo h\nErr.Raise 5\nDebug.Print \"x\"\nExit Sub\nh:\nDebug.Print \"first \" & Err.Number\nOn Error GoTo -1\nOn Error GoTo h2\nErr.Raise 6\nExit Sub\nh2:\nDebug.Print \"second \" & Err.Number\nEnd Sub\n",
            "first 5\nsecond 6\n",
        ),
        case(
            "resume_next_in_handler_cleanup",
            "Sub Main()\nOn Error GoTo h\nErr.Raise 5\nExit Sub\nh:\nOn Error Resume Next\nDim x As Long\nx = 1 / 0\nDebug.Print \"cleanup ok \" & Err.Number\nEnd Sub\n",
            "cleanup ok 11\n",
        ),
        case(
            "erl_reports_line",
            "Sub Main()\nOn Error GoTo h\n10 Dim x As Long\n20 x = 1 / 0\n30 Debug.Print \"no\"\nExit Sub\nh:\nDebug.Print \"erl=\" & Erl\nEnd Sub\n",
            "erl=20\n",
        ),
        case(
            "return_without_gosub",
            "Sub Main()\nOn Error GoTo h\nReturn\nExit Sub\nh:\nDebug.Print Err.Number\nEnd Sub\n",
            " 3 \n",
        ),
        case(
            "resume_without_error",
            "Sub Main()\nOn Error GoTo h\nResume Next\nExit Sub\nh:\nDebug.Print Err.Number & \" \" & Err.Description\nEnd Sub\n",
            "20 Resume without error\n",
        ),
        case(
            "type_mismatch",
            "Sub Main()\nOn Error Resume Next\nDim x As Long\nx = \"abc\"\nDebug.Print Err.Number & \" \" & Err.Description\nEnd Sub\n",
            "13 Type mismatch\n",
        ),
        case(
            "overflow",
            "Sub Main()\nOn Error Resume Next\nDim x As Integer\nx = 32767\nx = x + 1\nDebug.Print Err.Number & \" \" & x\nEnd Sub\n",
            "6 32767\n",
        ),
        case(
            "err_persists_until_cleared",
            "Sub Main()\nOn Error Resume Next\nDim x As Long\nx = 1 / 0\nDebug.Print \"a\" & Err.Number\nDebug.Print \"b\" & Err.Number\nErr.Clear\nDebug.Print \"c\" & Err.Number\nEnd Sub\n",
            "a11\nb11\nc0\n",
        ),
        case(
            "object_variable_not_set",
            "Sub Main()\nOn Error Resume Next\nDim c As Collection\nDebug.Print c.Count\nDebug.Print Err.Number & \" \" & Err.Description\nEnd Sub\n",
            "91 Object variable or With block variable not set\n",
        ),
    ]);
}
