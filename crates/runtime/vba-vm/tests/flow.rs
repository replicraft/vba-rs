//! End-to-end programs: control flow.

mod common;

use common::*;

#[test]
fn control_flow() {
    check(&[
        case(
            "if_else",
            "Sub Main()\nDim x As Long: x = 5\nIf x > 3 Then\nDebug.Print \"big\"\nElseIf x > 1 Then\nDebug.Print \"mid\"\nElse\nDebug.Print \"small\"\nEnd If\nEnd Sub\n",
            "big\n",
        ),
        case(
            "if_elseif_chain",
            "Sub Main()\nDim x As Long: x = 2\nIf x > 3 Then\nDebug.Print \"big\"\nElseIf x > 1 Then\nDebug.Print \"mid\"\nElse\nDebug.Print \"small\"\nEnd If\nEnd Sub\n",
            "mid\n",
        ),
        case(
            "single_line_if",
            "Sub Main()\nIf 1 = 1 Then Debug.Print \"y\" Else Debug.Print \"n\"\nIf 1 = 2 Then Debug.Print \"y\" Else Debug.Print \"n\"\nEnd Sub\n",
            "y\nn\n",
        ),
        case(
            "for_loop",
            "Sub Main()\nDim i As Long, s As Long\nFor i = 1 To 5\ns = s + i\nNext i\nDebug.Print \"s=\" & s & \" i=\" & i\nEnd Sub\n",
            "s=15 i=6\n",
        ),
        case(
            "for_step_negative",
            "Sub Main()\nDim i As Long, t As String\nFor i = 10 To 1 Step -3\nt = t & i & \",\"\nNext\nDebug.Print t\nEnd Sub\n",
            "10,7,4,1,\n",
        ),
        case(
            "for_zero_iterations",
            "Sub Main()\nDim i As Long\nFor i = 5 To 1\nDebug.Print \"never\"\nNext\nDebug.Print \"done\" & i\nEnd Sub\n",
            "done5\n",
        ),
        case(
            "for_double_step",
            "Sub Main()\nDim d As Double, n As Long\nFor d = 0 To 1 Step 0.25\nn = n + 1\nNext\nDebug.Print n\nEnd Sub\n",
            " 5 \n",
        ),
        case(
            "exit_for",
            "Sub Main()\nDim i As Long\nFor i = 1 To 10\nIf i = 4 Then Exit For\nNext\nDebug.Print i\nEnd Sub\n",
            " 4 \n",
        ),
        case(
            "nested_for_exit_inner",
            "Sub Main()\nDim i As Long, j As Long, c As Long\nFor i = 1 To 3\nFor j = 1 To 3\nIf j = 2 Then Exit For\nc = c + 1\nNext\nNext\nDebug.Print c\nEnd Sub\n",
            " 3 \n",
        ),
        case(
            "while_wend",
            "Sub Main()\nDim n As Long\nWhile n < 3\nn = n + 1\nWend\nDebug.Print n\nEnd Sub\n",
            " 3 \n",
        ),
        case(
            "do_while_loop",
            "Sub Main()\nDim n As Long\nDo While n < 3\nn = n + 1\nLoop\nDebug.Print n\nEnd Sub\n",
            " 3 \n",
        ),
        case(
            "do_loop_until",
            "Sub Main()\nDim n As Long\nDo\nn = n + 2\nLoop Until n >= 5\nDebug.Print n\nEnd Sub\n",
            " 6 \n",
        ),
        case(
            "do_until_exit_do",
            "Sub Main()\nDim n As Long\nDo Until False\nn = n + 1\nIf n = 7 Then Exit Do\nLoop\nDebug.Print n\nEnd Sub\n",
            " 7 \n",
        ),
        case(
            "select_case_values",
            "Function F(x As Long) As String\nSelect Case x\nCase 1\nF = \"one\"\nCase 2, 3\nF = \"two-three\"\nCase 4 To 6\nF = \"four-six\"\nCase Is > 10\nF = \"big\"\nCase Else\nF = \"other\"\nEnd Select\nEnd Function\nSub Main()\nDebug.Print F(1) & F(3) & F(5) & F(11) & F(8)\nEnd Sub\n",
            "onetwo-threefour-sixbigother\n",
        ),
        case(
            "select_case_strings",
            "Sub Main()\nDim s As String: s = \"b\"\nSelect Case s\nCase \"a\"\nDebug.Print 1\nCase \"b\"\nDebug.Print 2\nEnd Select\nEnd Sub\n",
            " 2 \n",
        ),
        case(
            "goto_label",
            "Sub Main()\nDim i As Long\ni = 0\ntop:\ni = i + 1\nIf i < 3 Then GoTo top\nDebug.Print i\nEnd Sub\n",
            " 3 \n",
        ),
        case(
            "on_goto",
            "Sub Main()\nDim k As Long\nFor k = 0 To 3\nOn k GoTo a, b\nDebug.Print \"fall\" & k\nGoTo done\na:\nDebug.Print \"A\"\nGoTo done\nb:\nDebug.Print \"B\"\ndone:\nNext\nEnd Sub\n",
            "fall0\nA\nB\nfall3\n",
        ),
        case(
            "gosub_return",
            "Sub Main()\nDim x As Long\nx = 1\nGoSub inc\nGoSub inc\nDebug.Print x\nExit Sub\ninc:\nx = x * 2 + 1\nReturn\nEnd Sub\n",
            " 7 \n",
        ),
        case(
            "on_gosub",
            "Sub Main()\nDim n As Long\nOn 2 GoSub one, two\nDebug.Print n\nExit Sub\none:\nn = n + 1\nReturn\ntwo:\nn = n + 10\nReturn\nEnd Sub\n",
            " 10 \n",
        ),
        case(
            "with_block",
            "Private Type P\nX As Long\nY As Long\nEnd Type\nSub Main()\nDim a As P\nWith a\n.X = 3\n.Y = 4\nEnd With\nDebug.Print a.X + a.Y\nEnd Sub\n",
            " 7 \n",
        ),
        case(
            "exit_sub",
            "Sub Main()\nDebug.Print 1\nExit Sub\nDebug.Print 2\nEnd Sub\n",
            " 1 \n",
        ),
        case(
            "end_statement_stops",
            "Sub Main()\nDebug.Print 1\nEnd\nDebug.Print 2\nEnd Sub\n",
            " 1 \n",
        ),
    ]);
}
