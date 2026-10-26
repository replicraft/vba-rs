' INV: vbal.3.2.2
' SOURCE: spec ms-vbal 3.2.2 logical line grammar, DBCS white space U+3000 is white space (WSC) separating tokens
' XFAIL: wrong not implemented: Unicode line terminators and space characters (the lexer is byte-based)
' EXPECT: 12
Public Sub Main()
    Dim　x　As　Long
    x　=　5　+　7
    Debug.Print　CStr(x)
End Sub
