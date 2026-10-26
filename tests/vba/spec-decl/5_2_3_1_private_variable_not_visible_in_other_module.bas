' INV: vbal.5.2.3.1
' SOURCE: spec ms-vbal 5.2.3.1 private variable only accessible within module
' ERROR: load
Public Sub Main()
    Other.hidden = 3
End Sub
'#MODULE Other standard
Private hidden As Long
