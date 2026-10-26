' INV: vbal.3.3.5.2
' SOURCE: spec ms-vbal 3.3.5.2 reserved identifiers, a reserved identifier cannot name a procedure
' ERROR: load
Public Sub Main()
    Close
End Sub
Public Sub Close()
End Sub
