' INV: vbal.6.1.2.11.1.15, func.instrrev
' SOURCE: spec ms-vbal 6.1.2.11.1.15 InStrRev returns Null when StringCheck or StringMatch is Null
' XFAIL: wrong pending
' EXPECT: b=True
' EXPECT: d=True
Public Sub Main()
    Debug.Print "b=" & CStr(IsNull(InStrRev(Null, "a")))
    Debug.Print "d=" & CStr(IsNull(InStrRev("abc", Null)))
End Sub
