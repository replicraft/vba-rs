' INV: func.csng
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: 75.34211
' EXPECT: 75.34216
Public Sub Main()
    Debug.Print CStr(CSng(75.3421115))
    Debug.Print CStr(CSng(75.3421555))
End Sub
