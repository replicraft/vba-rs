' INV: vbal.5.3.1.8
' SOURCE: spec ms-vbal 5.3.1.8 an event handler must be a subroutine declaration
' STRICT_SPEC: true
' ERROR: load
'#MODULE Src class
Public Event Fire()
Public Sub Go()
    RaiseEvent Fire
End Sub
'#MODULE Sink class
Private WithEvents w As Src
Private Function w_Fire() As Long
End Function
'#MODULE Module1 standard
Public Sub Main()
End Sub
