' INV: func.cbyte
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: 0
' EXPECT: 255
Public Sub Main()
    Debug.Print CStr(CByte(0))
    Debug.Print CStr(CByte(255))
End Sub
