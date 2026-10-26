' INV: func.instr, lit.null
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/instr-function
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(InStr(Null, "a"))
    Debug.Print IsNull(InStr("a", Null))
End Sub
