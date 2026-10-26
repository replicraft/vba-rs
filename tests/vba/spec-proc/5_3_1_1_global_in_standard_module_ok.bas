' INV: vbal.5.3.1.1
' SOURCE: spec ms-vbal 5.3.1.1 Global equals Public; default scope is Public
' EXPECT: g=1
' EXPECT: d=2
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print "g=" & CStr(Mod2.G())
    Debug.Print "d=" & CStr(Mod2.D())
End Sub
'#MODULE Mod2 standard
Global Function G() As Long
    G = 1
End Function
Function D() As Long
    D = 2
End Function
