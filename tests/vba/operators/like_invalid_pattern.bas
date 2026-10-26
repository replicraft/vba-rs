' INV: op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like operator invalid pattern
' ERROR: 93
Public Sub Main()
    Debug.Print "a [xyz" Like "a [*"
End Sub
