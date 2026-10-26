' INV: func.cbyte
' SOURCE: spec ms-vbal 5.5.1.2.1 numeric Let-coercion overflow
' ERROR: 6
Public Sub Main()
    Debug.Print CByte(256)
End Sub
