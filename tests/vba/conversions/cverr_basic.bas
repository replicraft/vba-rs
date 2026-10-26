' INV: func.cverr, func.iserror
' SOURCE: spec ms-vbal 6.1.2.3.1.14 CVErr
' EXPECT: True
' EXPECT: False
' EXPECT: Error 2042
Public Sub Main()
    Debug.Print CStr(IsError(CVErr(11)))
    Debug.Print CStr(IsError(5))
    Debug.Print CStr(CVErr(2042))
End Sub
