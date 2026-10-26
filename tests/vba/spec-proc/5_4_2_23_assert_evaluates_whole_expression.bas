' INV: vbal.5.4.2.23
' SOURCE: spec ms-vbal 5.4.2.23 the whole boolean expression of Debug.Assert is always evaluated
' EXPECT: probe-called=1
Dim called As Long
Function Probe() As Boolean
    called = called + 1
    Probe = False
End Function
Public Sub Main()
    Debug.Assert True Or Probe()
    Debug.Print "probe-called=" & CStr(called)
End Sub
