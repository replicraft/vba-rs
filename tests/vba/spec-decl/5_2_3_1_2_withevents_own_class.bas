' INV: vbal.5.2.3.1.2, decl.withevents
' SOURCE: spec ms-vbal 5.2.3.1.2 WithEvents type must not be the containing class
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
End Sub
'#MODULE Selfish class
Public Event Fired()
Private WithEvents me2 As Selfish
