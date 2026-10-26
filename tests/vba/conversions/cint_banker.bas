' INV: func.cint, func.clng
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: 0
' EXPECT: 2
' EXPECT: 2
' EXPECT: 0
' EXPECT: 2
' EXPECT: 4
Public Sub Main()
    Debug.Print CStr(CInt(0.5))
    Debug.Print CStr(CInt(1.5))
    Debug.Print CStr(CInt(2.5))
    Debug.Print CStr(CLng(0.5))
    Debug.Print CStr(CLng(1.5))
    Debug.Print CStr(CLng(3.5))
End Sub
