' INV: vbal.5.3.1.3
' SOURCE: derived 5.3.1.3 a procedure of the module is found before a library function of the same name; the library function stays reachable through its VBA qualifier
' METHOD: by hand: VBA.Left("abc", 1) is "a"
' EXPECT: mine
' EXPECT: a
Private Function Left(s As String, n As Long) As String
    Left = "mine"
End Function
Public Sub Main()
    Debug.Print Left("abc", 1)
    Debug.Print VBA.Left("abc", 1)
End Sub
