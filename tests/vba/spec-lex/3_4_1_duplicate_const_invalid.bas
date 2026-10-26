' INV: vbal.3.4.1, cc.const
' SOURCE: spec ms-vbal 3.4.1 conditional compilation const directive, the name of every #Const MUST be different within a module
' XFAIL: wrong pending
' ERROR: load
#Const A = 1
#Const A = 2
Public Sub Main()
End Sub
