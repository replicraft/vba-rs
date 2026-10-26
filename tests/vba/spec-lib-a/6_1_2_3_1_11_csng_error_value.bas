' INV: vbal.6.1.2.3.1.11, func.csng
' SOURCE: spec ms-vbal 6.1.2.3.1.11 CSng of an Error value converts its error code
' XFAIL: wrong pending
' EXPECT: err5=5
Public Sub Main()
    Debug.Print "err5=" & CStr(CSng(CVErr(5)))
End Sub
