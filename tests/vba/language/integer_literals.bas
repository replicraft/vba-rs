' INV: lit.integer
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: IntegerIntegerLongLongDouble
Public Sub Main()
    Debug.Print TypeName(1); TypeName(32767); TypeName(32768); TypeName(2147483647); TypeName(2147483648#)
End Sub
