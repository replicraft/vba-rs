' INV: vbal.5.6.1
' SOURCE: derived 5.6.1 a module name classifies as a module, not as a value: it cannot be printed or assigned
' METHOD: by construction
' ERROR: load
Public Sub Main()
    Debug.Print Module1
End Sub
