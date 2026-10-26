' INV: vbal.5.6.11
' SOURCE: spec ms-vbal 5.6.11 Me within a procedural module is invalid
' ERROR: load
Public Sub Main()
    Dim o As Object
    Set o = Me
End Sub
