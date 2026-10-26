' INV: func.clngptr
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: 5
Public Sub Main()
    Debug.Print CStr(CLngPtr(5))
End Sub
