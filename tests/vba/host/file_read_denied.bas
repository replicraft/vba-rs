' INV: stmt.open, func.dir
' SOURCE: derived deny by default: no FileRead grant is error 70
' ERROR: 70
Public Sub Main()
    Debug.Print Dir("/x")
End Sub
