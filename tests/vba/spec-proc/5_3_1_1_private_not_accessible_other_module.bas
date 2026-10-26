' INV: vbal.5.3.1.1
' SOURCE: spec ms-vbal 5.3.1.1 Private procedure accessible only within its module
' ERROR: load
'#MODULE Module1 standard
Public Sub Main()
    Hidden
End Sub
'#MODULE Mod2 standard
Private Sub Hidden()
End Sub
