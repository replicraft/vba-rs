' INV: vbal.6.1.2.3.1.5, func.cdbl
' SOURCE: spec ms-vbal 6.1.2.3.1.5 CDbl of an Error value converts its error code
' XFAIL: wrong pending
' EXPECT: err5=5
Public Sub Main()
    Debug.Print "err5=" & CStr(CDbl(CVErr(5)))
End Sub
