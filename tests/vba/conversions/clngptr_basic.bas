' INV: func.clngptr
' SOURCE: spec ms-vbal 6.1.2.3.1.10 CLngPtr
' EXPECT: 5
Public Sub Main()
    Debug.Print CStr(CLngPtr(5))
End Sub
