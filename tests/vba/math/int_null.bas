' INV: func.int, lit.null
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/int-fix-functions
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Int(Null))
End Sub
