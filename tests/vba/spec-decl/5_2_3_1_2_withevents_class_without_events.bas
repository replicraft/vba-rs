' INV: vbal.5.2.3.1.2, decl.withevents
' SOURCE: spec ms-vbal 5.2.3.1.2 WithEvents type must have at least one event
' XFAIL: wrong not implemented: static checks that a WithEvents class declares events and is not the containing class
' ERROR: load
Public Sub Main()
End Sub
'#MODULE Plain class
Public Sub Hi()
End Sub
'#MODULE Listener class
Private WithEvents p As Plain
