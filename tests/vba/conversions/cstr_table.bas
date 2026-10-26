' INV: func.cstr, lit.empty
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: True
' EXPECT: False
' EXPECT: 
' EXPECT: Error 11
' EXPECT: -5
Public Sub Main()
    Debug.Print CStr(CStr(True))
    Debug.Print CStr(CStr(False))
    Debug.Print CStr(CStr(Empty))
    Debug.Print CStr(CStr(CVErr(11)))
    Debug.Print CStr(CStr(-5))
End Sub
