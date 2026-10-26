' INV: vbal.5.4.3.5, stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 Mid$ mode specifier
' EXPECT: aXcde
Public Sub Main()
    Dim s As String
    s = "abcde"
    Mid$(s, 2, 1) = "X"
    Debug.Print s
End Sub
