' INV: vbal.4.1
' SOURCE: derived 4.1 a project is a set of modules; a Public member of one module is visible in the others, unqualified or qualified by the module name
' METHOD: by hand: Twice(5)=10, Lib.Twice(4)=8; Counter set to 3 then two Bump calls gives 5, seen through both spellings
' EXPECT:  10 
' EXPECT:  8 
' EXPECT:  5 
' EXPECT:  5 
'#MODULE Lib standard
Public Const K As Long = 5
Public Counter As Long
Public Function Twice(ByVal n As Long) As Long
    Twice = n * 2
End Function
Public Sub Bump()
    Counter = Counter + 1
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print Twice(K)
    Debug.Print Lib.Twice(4)
    Lib.Counter = 3
    Bump
    Lib.Bump
    Debug.Print Lib.Counter
    Debug.Print Counter
End Sub
