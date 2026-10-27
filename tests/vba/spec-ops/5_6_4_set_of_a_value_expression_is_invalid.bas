' INV: vbal.5.6.4
' SOURCE: derived 5.6.4 Set binds its right side as an object; a number is not an object
' METHOD: by construction
' ERROR: load
Public Sub Main()
    Dim o As Object
    Set o = 5
End Sub
