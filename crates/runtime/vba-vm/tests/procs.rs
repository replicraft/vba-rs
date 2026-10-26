//! End-to-end programs: procedures, arguments and module state.

mod common;

use common::*;

#[test]
fn procedures() {
    check(&[
        case(
            "recursion_factorial",
            "Function Fact(ByVal n As Long) As Long\nIf n <= 1 Then\nFact = 1\nElse\nFact = n * Fact(n - 1)\nEnd If\nEnd Function\nSub Main()\nDebug.Print \"f=\" & Fact(10)\nEnd Sub\n",
            "f=3628800\n",
        ),
        case(
            "recursion_fib",
            "Function Fib(n As Long) As Long\nIf n < 2 Then Fib = n Else Fib = Fib(n - 1) + Fib(n - 2)\nEnd Function\nSub Main()\nDebug.Print \"fib=\" & Fib(15)\nEnd Sub\n",
            "fib=610\n",
        ),
        case(
            "byref_swap",
            "Sub Swap(a As Long, b As Long)\nDim t As Long\nt = a: a = b: b = t\nEnd Sub\nSub Main()\nDim x As Long, y As Long\nx = 1: y = 2\nSwap x, y\nDebug.Print x & \",\" & y\nEnd Sub\n",
            "2,1\n",
        ),
        case(
            "byval_copy",
            "Sub Bump(ByVal a As Long)\na = a + 100\nEnd Sub\nSub Main()\nDim x As Long: x = 1\nBump x\nDebug.Print x\nEnd Sub\n",
            " 1 \n",
        ),
        case(
            "byref_expression_is_temp",
            "Sub Bump(a As Long)\na = a + 100\nEnd Sub\nSub Main()\nDim x As Long: x = 1\nBump x + 0\nDebug.Print x\nEnd Sub\n",
            " 1 \n",
        ),
        case(
            "byref_array_element",
            "Sub Inc(a As Long)\na = a + 1\nEnd Sub\nSub Main()\nDim v(1 To 3) As Long\nInc v(2)\nInc v(2)\nDebug.Print v(1) & v(2) & v(3)\nEnd Sub\n",
            "020\n",
        ),
        case(
            "byref_udt_field",
            "Private Type P\nX As Long\nEnd Type\nSub Inc(a As Long)\na = a + 5\nEnd Sub\nSub Main()\nDim p As P\nInc p.X\nDebug.Print p.X\nEnd Sub\n",
            " 5 \n",
        ),
        case(
            "byref_passthrough",
            "Sub Inner(a As Long)\na = a * 2\nEnd Sub\nSub Outer(a As Long)\nInner a\nInner a\nEnd Sub\nSub Main()\nDim x As Long: x = 3\nOuter x\nDebug.Print x\nEnd Sub\n",
            " 12 \n",
        ),
        case(
            "optional_default",
            "Function F(a As Long, Optional b As Long = 10) As Long\nF = a + b\nEnd Function\nSub Main()\nDebug.Print F(1) & \",\" & F(1, 2)\nEnd Sub\n",
            "11,3\n",
        ),
        case(
            "optional_variant_ismissing",
            "Function F(Optional v As Variant) As String\nIf IsMissing(v) Then F = \"missing\" Else F = \"got \" & v\nEnd Function\nSub Main()\nDebug.Print F() & \"/\" & F(7)\nEnd Sub\n",
            "missing/got 7\n",
        ),
        case(
            "named_arguments",
            "Function F(a As Long, b As Long, c As Long) As String\nF = a & \"-\" & b & \"-\" & c\nEnd Function\nSub Main()\nDebug.Print F(c:=3, a:=1, b:=2)\nEnd Sub\n",
            "1-2-3\n",
        ),
        case(
            "skipped_optional",
            "Function F(Optional a As Long = 1, Optional b As Long = 2) As Long\nF = a * 10 + b\nEnd Function\nSub Main()\nDebug.Print F(, 5)\nEnd Sub\n",
            " 15 \n",
        ),
        case(
            "paramarray",
            "Function Total(ParamArray v() As Variant) As Long\nDim i As Long, t As Long\nFor i = LBound(v) To UBound(v)\nt = t + v(i)\nNext\nTotal = t\nEnd Function\nSub Main()\nDebug.Print Total(1, 2, 3, 4) & \"/\" & Total()\nEnd Sub\n",
            "10/0\n",
        ),
        case(
            "function_result_default",
            "Function F() As String\nEnd Function\nFunction G() As Long\nEnd Function\nSub Main()\nDebug.Print \"[\" & F() & \"][\" & G() & \"]\"\nEnd Sub\n",
            "[][0]\n",
        ),
        case(
            "function_call_statement_discards",
            "Function F() As Long\nDebug.Print \"called\"\nF = 1\nEnd Function\nSub Main()\nF\nCall F\nEnd Sub\n",
            "called\ncalled\n",
        ),
        case(
            "static_local",
            "Sub Tick()\nStatic n As Long\nn = n + 1\nDebug.Print n\nEnd Sub\nSub Main()\nTick\nTick\nTick\nEnd Sub\n",
            " 1 \n 2 \n 3 \n",
        ),
        case(
            "module_variable",
            "Dim counter As Long\nSub Bump()\ncounter = counter + 1\nEnd Sub\nSub Main()\nBump\nBump\nDebug.Print counter\nEnd Sub\n",
            " 2 \n",
        ),
        case(
            "module_const_and_enum",
            "Const K As Long = 40\nEnum Color\nRed\nGreen = 5\nBlue\nEnd Enum\nSub Main()\nDebug.Print K + 2 & \" \" & Red & Green & Blue\nEnd Sub\n",
            "42 056\n",
        ),
        case(
            "nested_calls",
            "Function A() As Long\nA = 2\nEnd Function\nFunction B(x As Long) As Long\nB = x * 3\nEnd Function\nSub Main()\nDebug.Print B(A())\nEnd Sub\n",
            " 6 \n",
        ),
        case(
            "string_function_result",
            "Function Rep(s As String, n As Long) As String\nDim i As Long, r As String\nFor i = 1 To n\nr = r & s\nNext\nRep = r\nEnd Function\nSub Main()\nDebug.Print Rep(\"ab\", 3)\nEnd Sub\n",
            "ababab\n",
        ),
        case(
            "variant_argument_converted_to_long",
            "Function F(ByVal x As Long) As Long\nF = x + 1\nEnd Function\nSub Main()\nDim v As Variant: v = \"41\"\nDebug.Print F(v)\nEnd Sub\n",
            " 42 \n",
        ),
    ]);
}
