' INV: func.cverr, func.iserror
' SOURCE: derived CVErr builds an Error variant; IsError tests it
' EXPECT: True
' EXPECT: False
' EXPECT: Error 2042
Public Sub Main()
    Debug.Print CStr(IsError(CVErr(11)))
    Debug.Print CStr(IsError(5))
    Debug.Print CStr(CVErr(2042))
End Sub
