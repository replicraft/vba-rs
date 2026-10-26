' INV: func.strcomp, lit.null
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/strcomp-function
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(StrComp(Null, "a"))
End Sub
