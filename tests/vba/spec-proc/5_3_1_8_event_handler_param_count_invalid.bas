' INV: vbal.5.3.1.8
' SOURCE: spec ms-vbal 5.3.1.8 handler must have as many positional parameters as the event
' ERROR: load
'#MODULE Src class
Public Event Fire(ByVal n As Long)
Public Sub Go()
    RaiseEvent Fire(1)
End Sub
'#MODULE Sink class
Private WithEvents w As Src
Private Sub w_Fire()
End Sub
'#MODULE Module1 standard
Public Sub Main()
End Sub
