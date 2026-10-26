//! More end-to-end programs: statements and library calls not covered by
//! the other suites.

mod common;

use common::*;

#[test]
fn statements_and_operators() {
    check(&[
        case(
            "lset_rset",
            "Sub Main()\nDim s As String\ns = \"abcdef\"\nLSet s = \"XY\"\nDebug.Print \"[\" & s & \"]\"\ns = \"abcdef\"\nRSet s = \"XY\"\nDebug.Print \"[\" & s & \"]\"\ns = \"ab\"\nLSet s = \"WXYZ\"\nDebug.Print \"[\" & s & \"]\"\nEnd Sub\n",
            "[XY    ]\n[    XY]\n[WX]\n",
        ),
        case(
            "date_literals_and_arithmetic",
            "Sub Main()\nDim d As Date\nd = #3/15/2024#\nDebug.Print Format(d + 20, \"yyyy-mm-dd\")\nDebug.Print d < #4/1/2024#\nDebug.Print DateDiff(\"d\", #1/1/2024#, d)\nEnd Sub\n",
            "2024-04-04\nTrue\n 74 \n",
        ),
        case(
            "weekday_and_monthname",
            "Sub Main()\nDebug.Print Weekday(#3/15/2024#) & MonthName(3) & Format(#3/15/2024#, \"ddd\")\nEnd Sub\n",
            "6MarchFri\n",
        ),
        case(
            "byte_arithmetic_overflow",
            "Sub Main()\nDim b As Byte\nb = 250\nOn Error Resume Next\nb = b + 10\nDebug.Print Err.Number & \"/\" & b\nEnd Sub\n",
            "6/250\n",
        ),
        case(
            "long_overflow_multiplication",
            "Sub Main()\nDim a As Long\na = 100000\nOn Error Resume Next\nDebug.Print a * a\nDebug.Print Err.Number\nEnd Sub\n",
            " 6 \n",
        ),
        case(
            "variant_multiplication_promotes",
            "Sub Main()\nDim a As Variant\na = 100000\nDebug.Print a * a\nEnd Sub\n",
            " 10000000000 \n",
        ),
        case(
            "integer_division_by_zero",
            "Sub Main()\nOn Error Resume Next\nDim x As Long\nx = 5 \\ 0\nDebug.Print Err.Number\nErr.Clear\nx = 5 Mod 0\nDebug.Print Err.Number\nEnd Sub\n",
            " 11 \n 11 \n",
        ),
        case(
            "comparison_of_variant_string_and_number",
            "Sub Main()\nDim v As Variant\nv = \"10\"\nDebug.Print (v > 9) & (v = 10) & (\"10\" > \"9\")\nEnd Sub\n",
            "TrueFalseFalse\n",
        ),
        case(
            "null_propagation",
            "Sub Main()\nDim v As Variant\nv = Null\nDebug.Print IsNull(v + 1) & IsNull(v & \"a\") & IsNull(v = 1)\nDebug.Print (Null And False)\nEnd Sub\n",
            "TrueFalseTrue\nFalse\n",
        ),
        case(
            "eqv_imp_xor_bitwise",
            "Sub Main()\nDebug.Print (5 Xor 3) & \",\" & (True Eqv False) & \",\" & (False Imp True) & \",\" & (Not 0)\nEnd Sub\n",
            "6,False,True,-1\n",
        ),
        case(
            "string_multiplication_table_nested_loops",
            "Sub Main()\nDim i As Long, j As Long, s As String\nFor i = 1 To 3\ns = \"\"\nFor j = 1 To 3\ns = s & i * j & \" \"\nNext\nDebug.Print RTrim(s)\nNext\nEnd Sub\n",
            "1 2 3\n2 4 6\n3 6 9\n",
        ),
        case(
            "select_case_is_and_text_compare",
            "Option Compare Text\nSub Main()\nDim s As String\ns = \"ABC\"\nSelect Case s\nCase \"abc\"\nDebug.Print \"matched\"\nCase Else\nDebug.Print \"no\"\nEnd Select\nEnd Sub\n",
            "matched\n",
        ),
        case(
            "select_case_ranges_and_lists",
            "Function Kind(n As Long) As String\nSelect Case n\nCase Is < 0\nKind = \"neg\"\nCase 0\nKind = \"zero\"\nCase 1 To 9, 11 To 19\nKind = \"small\"\nCase 10\nKind = \"ten\"\nCase Else\nKind = \"big\"\nEnd Select\nEnd Function\nSub Main()\nDebug.Print Kind(-5) & Kind(0) & Kind(5) & Kind(10) & Kind(15) & Kind(99)\nEnd Sub\n",
            "negzerosmalltensmallbig\n",
        ),
        case(
            "with_nested_and_exit_inside",
            "Private Type A\nX As Long\nEnd Type\nPrivate Type B\nInn As A\nEnd Type\nSub Main()\nDim o As B\nWith o\nWith .Inn\n.X = 7\nEnd With\nEnd With\nDebug.Print o.Inn.X\nEnd Sub\n",
            " 7 \n",
        ),
        case(
            "do_loop_while_post_condition",
            "Sub Main()\nDim n As Long\nDo\nn = n + 1\nLoop While n < 3\nDebug.Print n\nEnd Sub\n",
            " 3 \n",
        ),
        case(
            "for_loop_variable_modified_in_body",
            "Sub Main()\nDim i As Long, c As Long\nFor i = 1 To 10\ni = i + 1\nc = c + 1\nNext\nDebug.Print c & \" \" & i\nEnd Sub\n",
            "5 11\n",
        ),
        case(
            "for_loop_bounds_evaluated_once",
            "Function Limit() As Long\nDebug.Print \"limit\"\nLimit = 3\nEnd Function\nSub Main()\nDim i As Long\nFor i = 1 To Limit()\nNext\nEnd Sub\n",
            "limit\n",
        ),
        case(
            "for_each_variant_array_modification_safe",
            "Sub Main()\nDim v As Variant, x As Variant, s As String\nv = Array(\"a\", \"b\", \"c\")\nFor Each x In v\ns = s & x\nNext\nDebug.Print s\nEnd Sub\n",
            "abc\n",
        ),
        case(
            "goto_out_of_loops",
            "Sub Main()\nDim i As Long, j As Long\nFor i = 1 To 3\nFor j = 1 To 3\nIf i * j = 4 Then GoTo done\nNext\nNext\ndone:\nDebug.Print i & j\nEnd Sub\n",
            "22\n",
        ),
        case(
            "nested_gosub",
            "Sub Main()\nDim t As String\nGoSub a\nDebug.Print t\nExit Sub\na:\nt = t & \"a\"\nGoSub b\nt = t & \"a\"\nReturn\nb:\nt = t & \"b\"\nReturn\nEnd Sub\n",
            "aba\n",
        ),
        case(
            "line_numbers_as_goto_targets",
            "Sub Main()\nDim n As Long\n10 n = n + 1\nIf n < 3 Then GoTo 10\nDebug.Print n\nEnd Sub\n",
            " 3 \n",
        ),
        case(
            "exit_function_with_result",
            "Function F(n As Long) As Long\nF = 1\nIf n > 0 Then\nF = 2\nExit Function\nEnd If\nF = 3\nEnd Function\nSub Main()\nDebug.Print F(1) & F(0)\nEnd Sub\n",
            "23\n",
        ),
        case(
            "optional_byref_modified",
            "Sub Bump(Optional ByRef x As Long = 5)\nx = x + 1\nDebug.Print x\nEnd Sub\nSub Main()\nDim a As Long\na = 10\nBump a\nBump\nDebug.Print a\nEnd Sub\n",
            " 11 \n 6 \n 11 \n",
        ),
        case(
            "paramarray_forwarding_count",
            "Function Cnt(ParamArray v() As Variant) As Long\nCnt = UBound(v) - LBound(v) + 1\nEnd Function\nSub Main()\nDebug.Print Cnt() & Cnt(1) & Cnt(1, 2, 3)\nEnd Sub\n",
            "013\n",
        ),
        case(
            "static_in_function_counts_across_calls",
            "Function Id() As Long\nStatic n As Long\nn = n + 1\nId = n\nEnd Function\nSub Main()\nDebug.Print Id() & Id() & Id()\nEnd Sub\n",
            "123\n",
        ),
        case(
            "recursive_with_arrays_hanoi_count",
            "Dim moves As Long\nSub Hanoi(n As Long, a As String, b As String, c As String)\nIf n = 0 Then Exit Sub\nHanoi n - 1, a, c, b\nmoves = moves + 1\nHanoi n - 1, c, b, a\nEnd Sub\nSub Main()\nHanoi 6, \"A\", \"B\", \"C\"\nDebug.Print moves\nEnd Sub\n",
            " 63 \n",
        ),
        case(
            "bubble_sort",
            "Sub Main()\nDim a(1 To 6) As Long, i As Long, j As Long, t As Long, s As String\na(1) = 5: a(2) = 3: a(3) = 9: a(4) = 1: a(5) = 7: a(6) = 2\nFor i = 1 To 5\nFor j = 1 To 6 - i\nIf a(j) > a(j + 1) Then\nt = a(j): a(j) = a(j + 1): a(j + 1) = t\nEnd If\nNext\nNext\nFor i = 1 To 6\ns = s & a(i)\nNext\nDebug.Print s\nEnd Sub\n",
            "123579\n",
        ),
        case(
            "sieve_of_eratosthenes",
            "Sub Main()\nDim p(2 To 30) As Boolean, i As Long, j As Long, s As String\nFor i = 2 To 30\np(i) = True\nNext\nFor i = 2 To 5\nIf p(i) Then\nFor j = i * i To 30 Step i\np(j) = False\nNext\nEnd If\nNext\nFor i = 2 To 30\nIf p(i) Then s = s & i & \" \"\nNext\nDebug.Print RTrim(s)\nEnd Sub\n",
            "2 3 5 7 11 13 17 19 23 29\n",
        ),
        case(
            "string_reverse_and_palindrome",
            "Function IsPal(s As String) As Boolean\nIsPal = (s = StrReverse(s))\nEnd Function\nSub Main()\nDebug.Print IsPal(\"level\") & IsPal(\"vba\")\nEnd Sub\n",
            "TrueFalse\n",
        ),
        case(
            "split_and_count_words",
            "Sub Main()\nDim w() As String, i As Long, n As Long\nw = Split(\"the quick  brown fox\", \" \")\nFor i = 0 To UBound(w)\nIf Len(w(i)) > 0 Then n = n + 1\nNext\nDebug.Print n & \"/\" & UBound(w)\nEnd Sub\n",
            "4/4\n",
        ),
        case(
            "math_functions",
            "Sub Main()\nDebug.Print Abs(-3) & Sgn(-2) & Int(3.7) & Round(2.5) & Round(3.5) & Round(2.675, 2)\nEnd Sub\n",
            "3-13242.68\n",
        ),
        case(
            "boolean_to_string_and_number",
            "Sub Main()\nDebug.Print CStr(True) & CInt(True) & CBool(\"False\") & (True + 1)\nEnd Sub\n",
            "True-1False0\n",
        ),
        case(
            "typename_of_values",
            "Sub Main()\nDebug.Print TypeName(1&) & TypeName(1#) & TypeName(\"s\") & TypeName(Nothing) & TypeName(Now) & TypeName(1.5!) & TypeName(CCur(1)) & TypeName(CByte(1))\nEnd Sub\n",
            "LongDoubleStringNothingDateSingleCurrencyByte\n",
        ),
        case(
            "vartype_values",
            "Sub Main()\nDim a(1) As Long\nDebug.Print VarType(1) & \",\" & VarType(\"s\") & \",\" & VarType(Null) & \",\" & VarType(a) & \",\" & VarType(Empty)\nEnd Sub\n",
            "2,8,1,8195,0\n",
        ),
        case(
            "isnumeric_isdate_isarray",
            "Sub Main()\nDim a(1) As Long\nDebug.Print IsNumeric(\"12\") & IsNumeric(\"x\") & IsDate(\"2024-01-02\") & IsArray(a) & IsObject(Nothing)\nEnd Sub\n",
            "TrueFalseTrueTrueTrue\n",
        ),
        case(
            "cvar_cdate_ccur",
            "Sub Main()\nDebug.Print CDbl(CCur(1.2345)) & \"|\" & Format(CDate(\"2024-05-06\"), \"d/m/yy\") & \"|\" & CLng(\"&H10\")\nEnd Sub\n",
            "1.2345|6/5/24|16\n",
        ),
        case(
            "instr_compare_modes",
            "Sub Main()\nDebug.Print InStr(1, \"Hello\", \"L\", vbTextCompare) & InStr(\"Hello\", \"L\") & InStr(3, \"Hello\", \"l\")\nEnd Sub\n",
            "303\n",
        ),
        case(
            "string_padding_and_formatting",
            "Sub Main()\nDebug.Print Format(5, \"000\") & \"|\" & Right(\"   \" & 7, 3) & \"|\" & Format(1234567.891, \"#,##0.0\") & \"|\" & Format(0.5, \"0.00%\")\nEnd Sub\n",
            "005|  7|1,234,567.9|50.00%\n",
        ),
        case(
            "ucase_lcase_proper",
            "Sub Main()\nDebug.Print UCase(\"abc\") & LCase(\"ABC\") & StrConv(\"hello world\", vbProperCase)\nEnd Sub\n",
            "ABCabcHello World\n",
        ),
        case(
            "join_filter",
            "Sub Main()\nDim a As Variant, f() As String\na = Array(\"apple\", \"banana\", \"cherry\")\nf = Filter(a, \"an\")\nDebug.Print UBound(f) & Join(f, \",\")\nEnd Sub\n",
            "0banana\n",
        ),
        case(
            "environment_independent_timer_and_now",
            "Sub Main()\nDebug.Print Format(Now, \"yyyy\") & (Timer >= 0)\nEnd Sub\n",
            "2000True\n",
        ),
    ]);
}

#[test]
fn printing_rules() {
    check(&[
        case(
            "number_formats",
            "Sub Main()\nDebug.Print 1; 2.5; -3; 1E+20; 0.1\nEnd Sub\n",
            " 1  2.5 -3  1E+20  0.1 \n",
        ),
        case(
            "booleans_dates_null_empty",
            "Sub Main()\nDebug.Print True; #1/2/2024#; Null; Empty; \"x\"\nEnd Sub\n",
            "True1/2/2024Nullx\n",
        ),
        case(
            "print_zones",
            "Sub Main()\nDebug.Print \"a\", \"b\", \"c\"\nDebug.Print 1, 2\nEnd Sub\n",
            "a             b             c\n 1             2 \n",
        ),
        case(
            "trailing_semicolon_continues_the_line",
            "Sub Main()\nDebug.Print \"a\";\nDebug.Print \"b\"\nDebug.Print\nDebug.Print \"c\"\nEnd Sub\n",
            "ab\n\nc\n",
        ),
        case(
            "tab_and_spc",
            "Sub Main()\nDebug.Print \"a\"; Tab(5); \"b\"; Spc(3); \"c\"\nEnd Sub\n",
            "a   b   c\n",
        ),
        case(
            "object_default_member_in_print",
            "Sub Main()\nOn Error Resume Next\nDim c As New Collection\nc.Add 7\nDebug.Print c(1)\nEnd Sub\n",
            " 7 \n",
        ),
    ]);
}
