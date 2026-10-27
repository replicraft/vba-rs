' INV: vbal.5.6.1
' SOURCE: derived 5.6.1 the name of an Enum classifies as a type; only its members are values
' METHOD: by construction
' ERROR: load
Public Enum Color
    Red = 1
End Enum
Public Sub Main()
    Debug.Print Color
End Sub
