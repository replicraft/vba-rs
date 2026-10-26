' INV: vbal.6.1.2.3.1.10, func.clngptr
' SOURCE: spec ms-vbal 6.1.2.3.1.10 CLngPtr of an Error value converts its error code
' XFAIL: wrong pending
' EXPECT: err3=3
Public Sub Main()
    Debug.Print "err3=" & CStr(CLngPtr(CVErr(3)))
End Sub
