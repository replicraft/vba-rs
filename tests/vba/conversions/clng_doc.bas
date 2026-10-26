' INV: func.clng
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: 25427
' EXPECT: 25428
Public Sub Main()
    Debug.Print CStr(CLng(25427.45))
    Debug.Print CStr(CLng(25427.55))
End Sub
