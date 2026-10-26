' INV: func.cint
' SOURCE: spec ms-vbal 6.1.2.3.1.7 CInt
' EXPECT: 2346
Public Sub Main()
    Debug.Print CStr(CInt(2345.5678))
End Sub
