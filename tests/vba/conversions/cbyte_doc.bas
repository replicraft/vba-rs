' INV: func.cbyte
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: 126
Public Sub Main()
    Debug.Print CStr(CByte(125.5678))
End Sub
