' INV: stmt.const
' SOURCE: spec ms-vbal 5.4.3.2 local Const declarations
' EXPECT:  3  6 hi
Public Sub Main()
    Const A = 3
    Const B As Long = A * 2
    Const S = "hi"
    Debug.Print A; B; S
End Sub
