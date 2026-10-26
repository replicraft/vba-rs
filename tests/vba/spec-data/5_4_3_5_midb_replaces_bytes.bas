' INV: vbal.5.4.3.5, stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 MidB works in bytes
' XFAIL: wrong pending
' EXPECT: Zbc
' EXPECT: aXc
Public Sub Main()
    Dim s As String
    s = "abc"
    MidB(s, 1, 2) = "Z"
    Debug.Print s
    s = "abc"
    MidB$(s, 3, 2) = "XY"
    Debug.Print s
End Sub
