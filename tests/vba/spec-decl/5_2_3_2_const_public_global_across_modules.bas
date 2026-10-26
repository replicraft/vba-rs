' INV: vbal.5.2.3.2
' SOURCE: spec ms-vbal 5.2.3.2 public constants accessible in project; private constants only within module
' EXPECT: 10 20 30
Public Sub Main()
    Debug.Print CStr(Other.P) & " " & CStr(Other.Doubled()) & " " & CStr(G)
End Sub
'#MODULE Other standard
Public Const P As Long = 10
Private Const Q As Long = 20
Global Const G As Long = 30
Public Function Doubled() As Long
    Doubled = Q
End Function
