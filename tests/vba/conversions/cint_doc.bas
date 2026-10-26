' INV: func.cint
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: 2346
Public Sub Main()
    Debug.Print CStr(CInt(2345.5678))
End Sub
