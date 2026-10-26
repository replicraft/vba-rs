' INV: vbal.5.5.2.2.1, stmt.set
' SOURCE: spec ms-vbal 5.5.2.2.1 Set-coercion to and from a class, a different class not implemented by the source is Type mismatch
' ERROR: 13
'#MODULE ClsA class
Public V As Long
'#MODULE ClsB class
Public W As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Dim a As ClsA
    Set o = New ClsB
    Set a = o
    Debug.Print "not reached"
End Sub
