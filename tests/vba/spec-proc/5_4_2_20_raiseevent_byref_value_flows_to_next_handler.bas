' INV: vbal.5.4.2.20, stmt.raiseevent
' SOURCE: spec ms-vbal 5.4.2.20 a ByRef event parameter carries its last value into the next handler invocation
' EXPECT: h1 sees 1
' EXPECT: h2 sees 11
' EXPECT: final=111
'#MODULE Src class
Public Event Fire(ByRef n As Long)
Public Function Go() As Long
    Dim v As Long
    v = 1
    RaiseEvent Fire(v)
    Go = v
End Function
'#MODULE Sink class
Private WithEvents w1 As Src
Private WithEvents w2 As Src
Public Function Run() As Long
    Dim s As New Src
    Set w1 = s
    Set w2 = s
    Run = s.Go()
End Function
Private Sub w1_Fire(ByRef n As Long)
    Debug.Print "h1 sees " & CStr(n)
    n = n + 10
End Sub
Private Sub w2_Fire(ByRef n As Long)
    Debug.Print "h2 sees " & CStr(n)
    n = n + 100
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim k As New Sink
    Debug.Print "final=" & CStr(k.Run())
End Sub
