' INV: obj.debug, func.spc, func.tab
' SOURCE: spec ms-vbal 5.4.5.8 Print statement Spc and Tab clauses
' EXPECT: a   b
' EXPECT: a   b
Public Sub Main()
    Debug.Print "a"; Spc(3); "b"
    Debug.Print "a"; Tab(5); "b"
End Sub
