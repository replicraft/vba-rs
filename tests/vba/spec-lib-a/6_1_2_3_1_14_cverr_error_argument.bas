' INV: vbal.6.1.2.3.1.14, func.cverr
' SOURCE: spec ms-vbal 6.1.2.3.1.14 CVErr of an Error value returns it without raising
' XFAIL: wrong pending
' EXPECT: pass=Error 7
' EXPECT: passIs=True
Public Sub Main()
    Debug.Print "pass=" & CStr(CVErr(CVErr(7)))
    Debug.Print "passIs=" & CStr(IsError(CVErr(CVErr(7))))
End Sub
