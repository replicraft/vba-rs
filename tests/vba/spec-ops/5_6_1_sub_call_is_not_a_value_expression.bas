' INV: vbal.5.6.1
' SOURCE: derived 5.6.1 a Sub has no result, so a call of it cannot be used where a value is required
' METHOD: by construction
' ERROR: load
Private Sub Act()
End Sub
Public Sub Main()
    Dim x As Long
    x = Act()
End Sub
