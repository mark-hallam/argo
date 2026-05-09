"""Minimal LangChain-style fixture for `argo genesis import` tests."""
from langchain import PromptTemplate

receptionist_prompt = PromptTemplate(
    template="""You are REX, a friendly clinic receptionist.
When a caller asks a question, acknowledge it before answering.
Question: {question}
Answer:""",
    input_variables=["question"],
)
