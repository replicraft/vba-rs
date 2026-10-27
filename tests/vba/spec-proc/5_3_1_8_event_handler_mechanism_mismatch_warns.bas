' INV: vbal.5.3.1.8
' SOURCE: spec ms-vbal 5.3.1.8 handler parameter mechanism must match the event's (spec-strict: a warning by default, the program runs)
' EXPECT: ok
'#MODULE Src class
Public Event Fire(ByRef n As Long)
Public Sub Go()
    Dim v As Long
    RaiseEvent Fire(v)
End Sub
'#MODULE Sink class
Private WithEvents w As Src
Private Sub w_Fire(ByVal n As Long)
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print "ok"
End Sub
