' INV: vbal.6.1.2.3.1.7, func.cint
' SOURCE: spec ms-vbal 6.1.2.3.1.7 CInt of an Error value converts its error code
' XFAIL: wrong pending
' EXPECT: err7=7
Public Sub Main()
    Debug.Print "err7=" & CStr(CInt(CVErr(7)))
End Sub
