' INV: vbal.5.2.3.1.2, decl.withevents
' SOURCE: spec ms-vbal 5.2.3.1.2 WithEvents type must have at least one event
' STRICT_SPEC: true
' ERROR: load
Public Sub Main()
End Sub
'#MODULE Plain class
Public Sub Hi()
End Sub
'#MODULE Listener class
Private WithEvents p As Plain
