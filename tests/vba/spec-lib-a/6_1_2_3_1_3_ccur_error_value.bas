' INV: vbal.6.1.2.3.1.3, func.ccur
' SOURCE: spec ms-vbal 6.1.2.3.1.3 CCur of an Error value converts its error code
' XFAIL: wrong pending
' EXPECT: err5=5
Public Sub Main()
    Debug.Print "err5=" & CStr(CCur(CVErr(5)))
End Sub
