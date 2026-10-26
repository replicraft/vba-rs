' INV: vbal.5.3.1.8, stmt.raiseevent
' SOURCE: spec ms-vbal 5.3.1.8 handler may use different parameter names and implicit/explicit mechanism
' EXPECT: got=5
'#MODULE Src class
Public Event Fire(ByRef n As Long)
Public Sub Go()
    Dim v As Long
    v = 5
    RaiseEvent Fire(v)
End Sub
'#MODULE Sink class
Private WithEvents w As Src
Private Sub w_Fire(other As Long)
    Debug.Print "got=" & CStr(other)
End Sub
Public Sub Run()
    Set w = New Src
    w.Go
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim s As New Sink
    s.Run
End Sub
