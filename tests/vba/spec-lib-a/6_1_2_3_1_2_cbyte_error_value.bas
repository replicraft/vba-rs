' INV: vbal.6.1.2.3.1.2, func.cbyte
' SOURCE: spec ms-vbal 6.1.2.3.1.2 CByte of an Error value converts its error code
' XFAIL: wrong pending
' EXPECT: err7=7
' EXPECT: err0=0
Public Sub Main()
    Debug.Print "err7=" & CStr(CByte(CVErr(7)))
    Debug.Print "err0=" & CStr(CByte(CVErr(0)))
End Sub
