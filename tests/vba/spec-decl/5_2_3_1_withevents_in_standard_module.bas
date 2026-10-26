' INV: vbal.5.2.3.1, decl.withevents
' SOURCE: spec ms-vbal 5.2.3.1 WithEvents not allowed in procedural module
' ERROR: load
Private WithEvents src As Source
Public Sub Main()
End Sub
'#MODULE Source class
Public Event Fired()
