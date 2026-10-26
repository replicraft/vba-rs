' INV: func.strcomp, lit.null
' SOURCE: spec ms-vbal 6.1.2.11.1.36 StrComp Null operand gives Null
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(StrComp(Null, "a"))
End Sub
