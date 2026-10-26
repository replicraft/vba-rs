' INV: func.cstr, lit.empty
' SOURCE: spec ms-vbal 6.1.2.3.1.12 CStr
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
