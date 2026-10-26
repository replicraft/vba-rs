' INV: func.cstr
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: 437.324
Public Sub Main()
    Debug.Print CStr(CStr(437.324))
End Sub
