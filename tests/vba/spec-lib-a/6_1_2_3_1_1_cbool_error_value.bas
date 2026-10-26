' INV: vbal.6.1.2.3.1.1, func.cbool
' SOURCE: spec ms-vbal 6.1.2.3.1.1 CBool of an Error value converts its error code
' XFAIL: wrong pending
' EXPECT: err0=False
' EXPECT: err5=True
Public Sub Main()
    Debug.Print "err0=" & CStr(CBool(CVErr(0)))
    Debug.Print "err5=" & CStr(CBool(CVErr(5)))
End Sub
